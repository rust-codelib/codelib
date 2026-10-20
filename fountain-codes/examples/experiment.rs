//! Finite, reproducible experiments. CSV stdout is raw data; stderr is metadata.
#[path = "support/sample_size.rs"]
mod sample_size;

use coding_core::IncrementalDecoder;
use fountain_codes::*;
use std::{
    collections::BTreeMap,
    error::Error,
    time::{Duration, Instant},
};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    fn index(&mut self, k: usize) -> usize {
        let k = k as u64;
        let threshold = 0u64.wrapping_sub(k) % k;
        loop {
            let v = self.next();
            if v >= threshold {
                return (v % k) as usize;
            }
        }
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn shuffle<T>(&mut self, a: &mut [T]) {
        for i in (1..a.len()).rev() {
            let j = self.index(i + 1);
            a.swap(i, j);
        }
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.iter().any(|s| s == "--help") {
        eprintln!("experimental: --profile gf256|gf65536 --k 12 --symbol-bytes W --message-bytes K*T --c 4 --instances 2 --trials 3 --seed 42 --mode sample|loss --epsilon 0.1 --loss 0.4 --repair-loss 0.1 --max-observations 100 --max-work 1000000000 --time-budget-seconds 60");
        return Ok(());
    }
    if args.len() % 2 != 0 {
        return Err("arguments must be --name value pairs".into());
    }
    let options: BTreeMap<_, _> = args
        .chunks_exact(2)
        .map(|p| (p[0].as_str(), p[1].as_str()))
        .collect();
    for key in options.keys() {
        if ![
            "--profile",
            "--k",
            "--symbol-bytes",
            "--message-bytes",
            "--c",
            "--instances",
            "--trials",
            "--seed",
            "--mode",
            "--epsilon",
            "--loss",
            "--repair-loss",
            "--max-observations",
            "--max-work",
            "--time-budget-seconds",
        ]
        .contains(key)
        {
            return Err(format!("unknown option {key}").into());
        }
    }
    let get = |key, default| options.get(key).copied().unwrap_or(default);
    let profile = match get("--profile", "gf256") {
        "gf256" => Profile::Gf256,
        "gf65536" => Profile::Gf65536,
        _ => return Err("unknown profile".into()),
    };
    let k: usize = get("--k", "12").parse()?;
    let t: usize = get(
        "--symbol-bytes",
        if profile == Profile::Gf256 { "1" } else { "2" },
    )
    .parse()?;
    let default_n = k.checked_mul(t).ok_or("message size overflow")?;
    let n: usize = options
        .get("--message-bytes")
        .map(|v| v.parse())
        .transpose()?
        .unwrap_or(default_n);
    let c: f64 = get("--c", "4").parse()?;
    let instances: usize = get("--instances", "2").parse()?;
    let trials: usize = get("--trials", "3").parse()?;
    let seed: u64 = get("--seed", "42").parse()?;
    let mode = get("--mode", "sample");
    // Preserve the decimal text in both the exact calculation and the CSV.
    let epsilon = get("--epsilon", "0.1");
    let loss: f64 = get("--loss", "0.4").parse()?;
    let repair_loss: f64 = get("--repair-loss", "0.1").parse()?;
    let max_observations: usize = get("--max-observations", "100").parse()?;
    let max_work: u128 = get("--max-work", "1000000000").parse()?;
    let seconds: u64 = get("--time-budget-seconds", "60").parse()?;
    if instances == 0
        || trials == 0
        || seconds == 0
        || !["sample", "loss"].contains(&mode)
        || !repair_loss.is_finite()
        || !(0.0..=1.0).contains(&repair_loss)
        || !loss.is_finite()
        || !(0.0..=1.0).contains(&loss)
    {
        return Err("invalid experimental parameter".into());
    }
    let d = logarithmic_draws(k, c)?;
    let config = RepairableConfig::try_new(profile, t, d, seed, ResourceLimits::default())?;
    let desc = config.descriptor(n, [0; 16])?;
    if k == 0 || desc.source_symbols() != k {
        return Err("K must be positive and equal ceil(N/T)".into());
    }
    let observations = instances
        .checked_mul(trials)
        .ok_or("observation overflow")?;
    if observations > max_observations {
        return Err("observation budget exceeded before starting".into());
    }
    // Conservative operation units, independent of timing and never an RSS claim.
    let l = desc.elements_per_symbol() as u128;
    let kk = k as u128;
    let work_per_observation = 2 * kk * kk * (kk + l) + 8 * kk * (d as u128 + 1) * (l + 1);
    let work = work_per_observation
        .checked_mul(observations as u128)
        .ok_or("work overflow")?;
    if work > max_work {
        return Err(format!("work budget exceeded: {work} > {max_work}").into());
    }
    let requested_m = sample_size::requested_sample_size(k, epsilon)?;
    eprintln!("experimental, profile={}, K={k}, N={n}, T={t}, d={d}, c={c}, n_encoded={}, instances={instances}, trials_per_instance={trials}, seed={seed}, mode={mode}, epsilon={epsilon}, loss={loss}, repair_loss={repair_loss}, max_observations={max_observations}, work_units={work}, max_work={max_work}, time_budget_seconds={seconds}, q_gt_k={}",profile.id(),2*k,(if profile==Profile::Gf256 {256} else {65536})>k);
    eprintln!("No confidence interval is claimed. Trials sharing a code seed are correlated; inspect per-instance rates. Memory columns are logical charges, not RSS. Timing is machine-dependent.");
    println!("profile,k,n,t,d,c,master_seed,instance,code_seed,trial,trial_seed,mode,epsilon,loss,m,rank,first_complete,decode_ok,decode_ns,decoder_logical_bytes,encoder_logical_bytes,catalog_logical_bytes,payload_bytes,wire_bytes,padded_source_bytes,target,repair_available,repair_ok,read_symbols,read_payload_bytes,disjoint_lower_bound,repair_ns,repair_loss,repair_m,global_ids,repair_available_ids,helper_ids");
    let started = Instant::now();
    let budget = Duration::from_secs(seconds);
    let mut master = Rng(seed);
    for instance in 0..instances {
        let code_seed = master.next();
        let mut source_rng = Rng(code_seed ^ 0x6d657373616765);
        let message: Vec<_> = (0..n).map(|_| source_rng.next() as u8).collect();
        let config =
            RepairableConfig::try_new(profile, t, d, code_seed, ResourceLimits::default())?;
        let encoder =
            RepairableEncoder::try_new(&message, (instance as u128).to_le_bytes(), config)?;
        let descriptor = encoder.descriptor().clone();
        let mut catalog = RepairCatalog::try_new(descriptor.clone(), ResourceLimits::default())?;
        for id in k as u64..(2 * k) as u64 {
            catalog.register(id)?;
        }
        let mut global_success = 0;
        let mut local_success = 0;
        for trial in 0..trials {
            if started.elapsed() >= budget {
                return Err(format!("time budget exhausted at instance={instance}, trial={trial}; completed CSV rows remain valid").into());
            }
            let trial_seed = master.next();
            let mut rng = Rng(trial_seed);
            let mut ids: Vec<_> = (0..(2 * k) as u64).collect();
            rng.shuffle(&mut ids);
            if mode == "sample" {
                ids.truncate(requested_m);
            } else {
                ids.retain(|_| rng.unit() >= loss);
            }
            let parts: Vec<_> = ids
                .iter()
                .map(|&id| encoder.symbol_unobserved(id))
                .collect::<Result<_, _>>()?;
            let mut decoder =
                RepairableDecoder::try_new(descriptor.clone(), ResourceLimits::default())?;
            let time = Instant::now();
            let mut first = None;
            for (index, part) in parts.iter().enumerate() {
                decoder.push_unobserved(part)?;
                if decoder.progress() == DecodeProgress::Complete && first.is_none() {
                    first = Some(index + 1);
                }
            }
            let decode_ns = time.elapsed().as_nanos();
            let decode_ok = decoder.message() == Some(message.as_slice());
            if decoder.message().is_some() && !decode_ok {
                return Err("decoded candidate differed from known original".into());
            }
            global_success += usize::from(decode_ok);
            // Both source and parity targets are sampled. The target and source
            // are excluded from the callback's accessible store by construction.
            let target = rng.index(2 * k) as u64;
            let expected = encoder.symbol_unobserved(target)?;
            let repair_ids: Vec<_> = (0..(2 * k) as u64)
                .filter(|&id| id != target && rng.unit() >= repair_loss)
                .collect();
            let store: BTreeMap<_, _> = repair_ids
                .iter()
                .map(|&id| Ok((id, StoredSymbol::new(encoder.symbol_unobserved(id)?))))
                .collect::<Result<_, FountainError>>()?;
            let available: Vec<_> = store.keys().copied().collect();
            let groups = catalog
                .availability(RepairRequest {
                    target,
                    available_ids: &available,
                })?
                .len();
            let mut reads = (0, 0);
            let mut helper_ids = Vec::new();
            let time = Instant::now();
            let restored = repair_local(
                &catalog,
                RepairRequest {
                    target,
                    available_ids: &available,
                },
                |r| {
                    helper_ids.push(r.helper_id());
                    store
                        .get(&r.helper_id())
                        .ok_or(FountainError::HelperUnavailable { id: r.helper_id() })?
                        .respond_unobserved(r)
                },
                &mut |e: &RepairEvent| {
                    if let RepairEvent::Completed {
                        read_symbols,
                        read_payload_bytes,
                        ..
                    } = e
                    {
                        reads = (*read_symbols, *read_payload_bytes);
                    }
                },
            )?;
            let repair_ns = time.elapsed().as_nanos();
            let repair_available = restored.is_some();
            let repair_ok = restored.as_ref() == Some(&expected);
            if repair_available && !repair_ok {
                return Err("repair differed from known original".into());
            }
            local_success += usize::from(repair_ok);
            let charge = ResourceCharge::calculate(
                &descriptor,
                ids.len(),
                catalog.entries(),
                catalog.edges(),
            )?;
            let first = first.map(|v| v.to_string()).unwrap_or_default();
            let list = |ids: &[u64]| ids.iter().map(u64::to_string).collect::<Vec<_>>().join(";");
            let global_ids = list(&ids);
            let repair_ids = list(&available);
            let helper_ids = list(&helper_ids);
            let repair_m = available.len();
            println!("{},{k},{n},{t},{d},{c},{seed},{instance},{code_seed},{trial},{trial_seed},{mode},{epsilon},{loss},{},{},{first},{decode_ok},{decode_ns},{},{},{},{},{},{},{target},{repair_available},{repair_ok},{},{},{groups},{repair_ns},{repair_loss},{repair_m},{global_ids},{repair_ids},{helper_ids}",profile.id(),ids.len(),decoder.rank(),charge.decoder_bytes,charge.encoder_bytes,charge.catalog_bytes,ids.len()*t,48+ids.len()*(64+t),k*t,reads.0,reads.1);
        }
        eprintln!("instance={instance}, code_seed={code_seed}, observations={trials}, decode_success={global_success}, rank_deficient={}, repair_success={local_success}",trials-global_success);
    }
    Ok(())
}
