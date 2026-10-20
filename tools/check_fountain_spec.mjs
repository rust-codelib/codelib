// Independent offline checks for the v1 specification. No codec or dependencies.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const vectors = JSON.parse(readFileSync(new URL('../docs/fixtures/fountain-v1.json', import.meta.url), 'utf8'));
const mask = (1n << 64n) - 1n;
const wordHex = value => value.toString(16).padStart(16, '0');
const mix = value => {
  value = ((value ^ (value >> 30n)) * 0xbf58476d1ce4e5b9n) & mask;
  value = ((value ^ (value >> 27n)) * 0x94d049bb133111ebn) & mask;
  return value ^ (value >> 31n);
};
function randomWords(initial) {
  let state = initial;
  return () => {
    state = (state + 0x9e3779b97f4a7c15n) & mask;
    return mix(state);
  };
}

// Form the whole GF(2)[x] product before long division. The Python reference
// instead reduces after every shift. Maximum intermediate degree is 30.
function multiply(a, b, width) {
  let product = 0;
  for (let i = 0; i < width * 8; i++) {
    if ((b >>> i) & 1) product ^= a * (2 ** i);
  }
  const modulus = width === 1 ? 0x11d : 0x1002d;
  for (let degree = 2 * width * 8 - 2; degree >= width * 8; degree--) {
    if ((product >>> degree) & 1) product ^= modulus * (2 ** (degree - width * 8));
  }
  return product;
}
function fieldSum(terms, width) {
  return terms.reduce((sum, [a, b]) => sum ^ multiply(a, b, width), 0);
}
function encodeWords(values, width) {
  const buffer = Buffer.alloc(values.length * width);
  values.forEach((value, i) => width === 1 ? buffer.writeUInt8(value, i) : buffer.writeUInt16LE(value, 2 * i));
  return buffer;
}
function supportFor(seed, id, k, draws, width) {
  if (id < BigInt(k)) return { support: [[Number(id), 1]] };
  const initialIndex = mix(seed ^ mix(id) ^ 0x524643315f494458n);
  const initialCoeff = mix(seed ^ mix(id) ^ 0x524643315f434f46n);
  const indexRandom = randomWords(initialIndex), coeffRandom = randomWords(initialCoeff);
  const threshold = (1n << 64n) % BigInt(k);
  const selected = [], indexWords = [];
  while (selected.length < draws) {
    const word = indexRandom();
    indexWords.push(wordHex(word));
    if (word >= threshold) selected.push(Number(word % BigInt(k)));
  }
  const distinct = [...new Set(selected)].sort((a, b) => a - b);
  const coeffWords = distinct.map(() => coeffRandom());
  const coefficients = coeffWords.map(word => Number(word & ((1n << BigInt(8 * width)) - 1n)));
  return {
    index_initial_state: wordHex(initialIndex), coefficient_initial_state: wordHex(initialCoeff),
    index_words: indexWords, draws: selected, distinct_indices: distinct,
    coefficient_words: coeffWords.map(wordHex),
    support: distinct.flatMap((index, i) => coefficients[i] ? [[index, coefficients[i]]] : []),
  };
}

const words = randomWords(0n);
assert.deepEqual(vectors.splitmix64.words_hex, Array.from({ length: 8 }, () => wordHex(words())));
assert.deepEqual(vectors.rejection_sampling.input_words_hex.flatMap(hex => {
  const word = BigInt('0x' + hex);
  return word < 1n ? [] : [Number(word % 3n)];
}), [1, 2, 0]);
assert.equal(multiply(0x80, 2, 1), 0x1d);
assert.equal(multiply(0x8000, 2, 2), 0x2d);
assert.equal(multiply(0xf2, 0xb5, 1), 1);

let symbolsChecked = 0;
for (const c of vectors.messages) {
  const desc = Buffer.from(c.descriptor_hex, 'hex');
  // Serialize from fields independently of Python's struct.pack format string.
  const encoded = Buffer.alloc(48);
  encoded.write('RFCD', 0, 'ascii');
  encoded.writeUInt16LE(1, 4); encoded.writeUInt16LE(c.profile, 6);
  Buffer.from(c.block_id_hex, 'hex').copy(encoded, 8);
  encoded.writeUInt16LE(c.n, 24); encoded.writeUInt16LE(c.t, 26);
  encoded.writeUInt16LE(c.k, 28); encoded.writeUInt16LE(c.d, 30);
  encoded.writeBigUInt64LE(BigInt('0x' + c.seed_hex), 32);
  encoded.writeUInt32LE(c.width === 1 ? 0x11d : 0x1002d, 40);
  assert.deepEqual(encoded, desc);
  assert.equal(c.k, Math.ceil(c.n / c.t)); assert.equal(c.l, c.t / c.width);
  assert.equal(c.padded_hex, c.message_hex.padEnd(c.k * c.t * 2, '0'));
  assert.equal(encodeWords(c.source_values.flat(), c.width).toString('hex'), c.padded_hex);

  for (const symbol of c.symbols) {
    const trace = supportFor(BigInt('0x' + c.seed_hex), BigInt(symbol.id), c.k, c.d, c.width);
    for (const [key, value] of Object.entries(trace)) assert.deepEqual(symbol[key], value);
    const values = Array.from({ length: c.l }, (_, col) => fieldSum(
      trace.support.map(([index, coefficient]) => [coefficient, c.source_values[index][col]]), c.width));
    assert.deepEqual(symbol.values, values);
    assert.equal(encodeWords(values, c.width).toString('hex'), symbol.payload_hex);
    const header = Buffer.alloc(64);
    header.write('RFCS', 0, 'ascii'); desc.copy(header, 4);
    header.writeBigUInt64LE(BigInt(symbol.id), 52); header.writeUInt16LE(c.t, 60);
    assert.equal(Buffer.concat([header, Buffer.from(symbol.payload_hex, 'hex')]).toString('hex'), symbol.packet_hex);
    symbolsChecked++;
  }
  // Nonzero 3x3 determinant certifies rank independently of Gaussian elimination.
  const a = c.global_recovery.coefficient_rows;
  const permutations = [[0,1,2],[0,2,1],[1,0,2],[1,2,0],[2,0,1],[2,1,0]];
  const determinant = permutations.reduce((sum, p) => sum ^ multiply(
    multiply(a[0][p[0]], a[1][p[1]], c.width), a[2][p[2]], c.width), 0);
  assert.notEqual(determinant, 0);
  assert.equal(c.global_recovery.rank, 3);
  const x = c.global_recovery.solution;
  for (let i = 0; i < 3; i++) {
    assert.deepEqual(c.global_recovery.rhs[i], Array.from({ length: c.l }, (_, col) => fieldSum(
      a[i].map((value, j) => [value, x[j][col]]), c.width)));
  }
  assert.equal(encodeWords(x.flat(), c.width).subarray(0, c.n).toString('hex'), c.message_hex);

  const repair = c.local_repair;
  assert(!repair.helpers_and_weights.some(([id]) => id === repair.target));
  const helpers = new Map(c.symbols.filter(s => s.id !== repair.target).map(s => [s.id, s]));
  const repaired = Array.from({ length: c.l }, (_, col) => fieldSum(
    repair.helpers_and_weights.map(([id, weight]) => [weight, helpers.get(id).values[col]]), c.width));
  assert.equal(encodeWords(repaired, c.width).toString('hex'), repair.payload_hex);
  // The weighted helper equations must equal the unit row of the target.
  for (let col = 0; col < c.k; col++) {
    assert.equal(fieldSum(repair.helpers_and_weights.map(([id, weight]) => {
      const entry = helpers.get(id).support.find(([index]) => index === col);
      return [weight, entry?.[1] ?? 0];
    }), c.width), col === Number(repair.target) ? 1 : 0);
  }
  assert.equal(repair.read_symbols, repair.helpers_and_weights.length);
  assert.equal(repair.read_payload_bytes, repair.read_symbols * c.t);
}

const zero = vectors.zero_parity;
const zeroTrace = supportFor(BigInt('0x' + zero.seed_hex), BigInt(zero.id), zero.k, zero.d, 1);
for (const [key, value] of Object.entries(zeroTrace)) assert.deepEqual(zero[key], value);
assert.deepEqual(zero.support, []); assert.equal(zero.payload_hex, '00');

const odd = vectors.odd_characteristic_system;
odd.equations.forEach((equation, index) => {
  const actual = Array.from({ length: 2 }, (_, col) => equation.a.reduce(
    (sum, a, row) => sum + a * odd.solution[row][col], 0) % 5);
  if (index < 3) assert.deepEqual(actual, equation.rhs);
  else assert.equal(actual.findIndex((v, col) => v !== equation.rhs[col]), equation.rhs_column);
});

for (const c of vectors.resource_cases) {
  const k = Math.ceil(c.n / c.t), l = c.t / c.width, s = Math.min(k, c.d);
  const scratch = k ? 192 + 16 * c.d + 16 * s + 2 * k + 4 * l + c.t : 0;
  assert.equal(scratch, c.scratch_bytes);
  assert.equal(48 + 2 * k * l + scratch, c.encoder_bytes);
  assert.equal(48 + 64 * k + 2 * k * l + c.n + scratch
    + 2 * k * (k + l) + c.unique_symbols * (32 + c.t), c.decoder_bytes);
  assert.equal(48 + 64 * k + 64 * c.catalog_entries + 48 * c.catalog_edges + scratch, c.catalog_bytes);
}
console.log(`Independent v1 checks passed: ${symbolsChecked} symbols, two profiles, wire, PRNG, determinant ranks, repair, GF(5), resource charges`);
