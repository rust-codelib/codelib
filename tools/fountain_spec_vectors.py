#!/usr/bin/env python3
"""Independent, dependency-free arithmetic reference for the fountain v1 spec.

This is a specification aid, not the Rust codec or a statistical experiment.
Run --check to verify committed vectors, --write to regenerate them explicitly.
"""

import argparse
import json
import struct
from pathlib import Path

MASK = (1 << 64) - 1
GAMMA = 0x9E3779B97F4A7C15
DOMAIN_INDEX = 0x524643315F494458  # ASCII RFC1_IDX, written as an integer.
DOMAIN_COEFF = 0x524643315F434F46  # ASCII RFC1_COF.
ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "docs/fixtures/fountain-v1.json"


def mix64(z):
    z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK
    z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK
    return z ^ (z >> 31)


class SplitMix64:
    def __init__(self, state):
        self.state = state

    def next(self):
        self.state = (self.state + GAMMA) & MASK
        return mix64(self.state)


def field_mul(a, b, width):
    modulus = {1: 0x11D, 2: 0x1002D}[width]
    order = 1 << (8 * width)
    result = 0
    while b:
        if b & 1:
            result ^= a
        b >>= 1
        a <<= 1
        if a & order:
            a ^= modulus
    return result


def field_inv(a, width):
    assert a != 0
    exponent = (1 << (8 * width)) - 2
    result = 1
    while exponent:
        if exponent & 1:
            result = field_mul(result, a, width)
        a = field_mul(a, a, width)
        exponent >>= 1
    return result


def unpack(payload, width):
    assert len(payload) % width == 0
    return [int.from_bytes(payload[i:i + width], "little")
            for i in range(0, len(payload), width)]


def pack(values, width):
    return b"".join(value.to_bytes(width, "little") for value in values)


def equation(seed, symbol_id, k, d, width):
    assert k > 0 and 1 <= d <= 65535
    if symbol_id < k:
        return {"support": [[symbol_id, 1]]}
    index_state = mix64(seed ^ mix64(symbol_id) ^ DOMAIN_INDEX)
    coeff_state = mix64(seed ^ mix64(symbol_id) ^ DOMAIN_COEFF)
    index_rng, coeff_rng = SplitMix64(index_state), SplitMix64(coeff_state)
    threshold = (1 << 64) % k
    draws, index_words = [], []
    while len(draws) < d:
        word = index_rng.next()
        index_words.append(f"{word:016x}")
        if word >= threshold:
            draws.append(word % k)
    distinct = sorted(set(draws))
    coeff_words = [coeff_rng.next() for _ in distinct]
    coefficient_mask = (1 << (8 * width)) - 1
    support = [[i, word & coefficient_mask]
               for i, word in zip(distinct, coeff_words)
               if word & coefficient_mask]
    return {
        "index_initial_state": f"{index_state:016x}",
        "coefficient_initial_state": f"{coeff_state:016x}",
        "index_words": index_words,
        "draws": draws,
        "distinct_indices": distinct,
        "coefficient_words": [f"{word:016x}" for word in coeff_words],
        "support": support,
    }


def descriptor(profile, block_id, n, t, seed, d):
    width = {1: 2, 2: 1}[profile]
    modulus = {1: 0x1002D, 2: 0x11D}[profile]
    assert len(block_id) == 16 and 0 <= n <= 4096
    assert 1 <= t <= 4096 and t % width == 0
    k = (n + t - 1) // t
    assert (k == 0 and d == 0) or (k > 0 and 1 <= d <= 65535)
    encoded = struct.pack("<4sHH16sHHHHQII", b"RFCD", 1, profile,
                          block_id, n, t, k, d, seed, modulus, 0)
    assert len(encoded) == 48
    return encoded


def packet(desc, symbol_id, payload):
    encoded = b"RFCS" + desc + struct.pack("<QHH", symbol_id, len(payload), 0) + payload
    assert len(encoded) == 64 + len(payload)
    return encoded


def solve(rows, rhs, width):
    """Batch Gauss-Jordan reference, deliberately not an incremental basis."""
    a, b = [row[:] for row in rows], [row[:] for row in rhs]
    k, l = len(a[0]), len(b[0])
    pivots = []
    rank = 0
    for col in range(k):
        pivot = next((i for i in range(rank, len(a)) if a[i][col]), None)
        if pivot is None:
            continue
        a[rank], a[pivot] = a[pivot], a[rank]
        b[rank], b[pivot] = b[pivot], b[rank]
        inverse = field_inv(a[rank][col], width)
        a[rank] = [field_mul(v, inverse, width) for v in a[rank]]
        b[rank] = [field_mul(v, inverse, width) for v in b[rank]]
        for i in range(len(a)):
            if i == rank:
                continue
            factor = a[i][col]
            a[i] = [x ^ field_mul(factor, y, width) for x, y in zip(a[i], a[rank])]
            b[i] = [x ^ field_mul(factor, y, width) for x, y in zip(b[i], b[rank])]
        pivots.append(col)
        rank += 1
    for i in range(rank, len(a)):
        if any(b[i]):
            return rank, None, True
    if rank < k:
        return rank, None, False
    solution = [[0] * l for _ in range(k)]
    for i, col in enumerate(pivots):
        solution[col] = b[i]
    return rank, solution, False


def message_case(name, profile, source, t, seed=0x0123456789ABCDEF, d=5):
    width = {1: 2, 2: 1}[profile]
    k, l = (len(source) + t - 1) // t, t // width
    padded = source + b"\0" * (k * t - len(source))
    chunks = [unpack(padded[i:i + t], width) for i in range(0, len(padded), t)]
    block_id = bytes(range(16))
    desc = descriptor(profile, block_id, len(source), t, seed, d)
    symbols = []
    for symbol_id in [0, 1, 2, 3, 4, 5, MASK]:
        trace = equation(seed, symbol_id, k, d, width)
        values = [0] * l
        for index, coeff in trace["support"]:
            for col in range(l):
                values[col] ^= field_mul(coeff, chunks[index][col], width)
        payload = pack(values, width)
        symbols.append({"id": symbol_id, **trace, "values": values,
                        "payload_hex": payload.hex(),
                        "packet_hex": packet(desc, symbol_id, payload).hex()})
    by_id = {symbol["id"]: symbol for symbol in symbols}
    parity_ids = [3, 4, 5]
    rows = [[0] * k for _ in parity_ids]
    rhs = []
    for row, symbol_id in zip(rows, parity_ids):
        for index, coeff in by_id[symbol_id]["support"]:
            row[index] = coeff
        rhs.append(by_id[symbol_id]["values"])
    rank, solution, inconsistent = solve(rows, rhs, width)
    assert rank == k and not inconsistent and solution == chunks
    # Source 0 is absent. Repair reads only the chosen parity and other sources.
    candidates = []
    for symbol_id in parity_ids:
        support = dict(by_id[symbol_id]["support"])
        if 0 in support:
            helpers = [index for index in support if index != 0] + [symbol_id]
            candidates.append((len(helpers), symbol_id, sorted(helpers)))
    _, chosen, helpers = min(candidates)
    support = dict(by_id[chosen]["support"])
    inverse = field_inv(support[0], width)
    weights = [[helper, inverse if helper == chosen
                else field_mul(inverse, support[helper], width)] for helper in helpers]
    repaired = [0] * l
    for helper, weight in weights:
        for col, value in enumerate(by_id[helper]["values"]):
            repaired[col] ^= field_mul(weight, value, width)
    assert repaired == chunks[0]
    return {
        "name": name, "profile": profile, "width": width, "n": len(source),
        "t": t, "k": k, "l": l, "seed_hex": f"{seed:016x}", "d": d,
        "block_id_hex": block_id.hex(), "message_hex": source.hex(),
        "padded_hex": padded.hex(), "source_values": chunks,
        "descriptor_hex": desc.hex(),
        "symbols": [{**symbol, "id": str(symbol["id"])} for symbol in symbols],
        "global_recovery": {"ids": [str(i) for i in parity_ids], "coefficient_rows": rows,
                            "rhs": rhs, "rank": rank, "solution": solution,
                            "insertion_pivot_columns": [1, 0, 2],
                            "message_hex": pack([v for row in solution for v in row], width)[:len(source)].hex()},
        "local_repair": {"target": "0", "parity_id": str(chosen),
                         "helpers_and_weights": [[str(i), w] for i, w in weights],
                         "payload_hex": pack(repaired, width).hex(),
                         "read_symbols": len(helpers), "read_payload_bytes": len(helpers) * t},
    }


def resource_charge(n, t, width, d, rank, unique_symbols, entries=0, edges=0):
    k, l = (n + t - 1) // t, t // width
    s = min(d, k)
    scratch = 0 if k == 0 else 128 + 16 * d + 16 * s + 2 * k + 4 * l + 64 + t
    return {
        "n": n, "t": t, "width": width, "d": d, "k": k, "l": l,
        "rank": rank, "unique_symbols": unique_symbols,
        "catalog_entries": entries, "catalog_edges": edges,
        "scratch_bytes": scratch,
        "encoder_bytes": 48 + 2 * k * l + scratch,
        "decoder_bytes": 48 + 64 * k + 2 * k * l + n + scratch
                         + 2 * k * (k + l) + unique_symbols * (32 + t),
        "catalog_bytes": 48 + 64 * k + 64 * entries + 48 * edges + scratch,
    }


def make_vectors():
    rng = SplitMix64(0)
    first_words = [f"{rng.next():016x}" for _ in range(8)]
    assert first_words[:3] == ["e220a8397b1dcdaf", "6e789e6aa1b965f4", "06c45d188009454f"]
    assert field_mul(0x80, 2, 1) == 0x1D
    assert field_mul(0x8000, 2, 2) == 0x2D
    seed = next(seed for seed in range(65536)
                if not equation(seed, 1, 1, 1, 1)["support"])
    zero_trace = equation(seed, 1, 1, 1, 1)
    cases = [message_case("gf256_three_parts", 2, bytes.fromhex("80ff341200"), 2),
             message_case("gf65536_odd_message", 1, bytes.fromhex("8001ff0034120080aa55cc"), 4)]
    odd_solution = [[2, 0], [4, 3]]
    for a, rhs in [([1, 1], [1, 3]), ([2, 1], [3, 3]), ([3, 2], [4, 1])]:
        assert [sum(a[i] * odd_solution[i][col] for i in range(2)) % 5
                for col in range(2)] == rhs
    budget_cases = [resource_charge(4096, 64, 2, 16, 64, 64),
                    resource_charge(4096, 1, 1, 256, 4096, 16384, 16384, 1048576),
                    resource_charge(4096, 2, 2, 256, 2048, 16384),
                    resource_charge(4096, 4096, 1, 256, 1, 16384)]
    assert budget_cases[1]["decoder_bytes"] < 64 * 1024 * 1024
    assert budget_cases[1]["catalog_bytes"] < 64 * 1024 * 1024
    assert budget_cases[-1]["decoder_bytes"] > 64 * 1024 * 1024
    assert resource_charge(4096, 1, 1, 256, 0, 0)["decoder_bytes"] == \
        resource_charge(4096, 1, 1, 256, 4096, 0)["decoder_bytes"]
    budget_cases.append(resource_charge(4096, 64, 2, 16, 0, 0))
    return {
        "spec": "codelib-repairable-fountain-v1",
        "splitmix64": {"initial_state_hex": "0000000000000000", "words_hex": first_words},
        "rejection_sampling": {"k": 3, "threshold": 1,
                               "input_words_hex": [f"{x:016x}" for x in [0, 1, 2, MASK]],
                               "accepted_indices": [1, 2, 0], "rejected_words": 1},
        "zero_parity": {"profile": 2, "k": 1, "d": 1, "id": "1",
                        "seed_hex": f"{seed:016x}", **zero_trace, "payload_hex": "00"},
        "empty_descriptor_hex": descriptor(1, bytes(16), 0, 64, 0, 0).hex(),
        "messages": cases,
        "resource_cases": budget_cases,
        "odd_characteristic_system": {
            "field": "GF(5)", "unknowns": 2, "rhs_columns": 2,
            "equations": [
                {"a": [1, 1], "rhs": [1, 3], "result": "Independent", "pivot_column": 0, "rank": 1},
                {"a": [2, 1], "rhs": [3, 3], "result": "Independent", "pivot_column": 1, "rank": 2},
                {"a": [3, 2], "rhs": [4, 1], "result": "Dependent", "rank": 2},
                {"a": [3, 2], "rhs": [4, 2], "result": "Inconsistent", "rhs_column": 1, "rank": 2}],
            "solution": [[2, 0], [4, 3]],
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group()
    action.add_argument("--write", action="store_true", help="explicitly regenerate fixtures")
    action.add_argument("--check", action="store_true", help="check fixtures (the default)")
    args = parser.parse_args()
    result = make_vectors()
    if args.write:
        FIXTURE.parent.mkdir(parents=True, exist_ok=True)
        FIXTURE.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
        print(f"Wrote {FIXTURE.relative_to(ROOT)}")
    else:
        expected = json.loads(FIXTURE.read_text(encoding="utf-8"))
        if result != expected:
            raise SystemExit("Vectors differ; inspect the spec and implementations before using --write")
        print("Fountain v1 vectors match: PRNG, rejection, zero parity, wire, two fields, decoding and repair")


if __name__ == "__main__":
    main()
