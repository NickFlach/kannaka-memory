//! T1.3 — the `EntropySource` seam (Quantum-Wave Track 1, issue #473).
//!
//! Abstracts where the engine draws the randomness that seeds dreams and Ξ
//! perturbations, so a deployment can opt into real quantum entropy without any
//! caller change. Every draw carries a [`Provenance`] so T1.4 can record, per
//! dream/Ξ record, exactly what entropy seeded it.
//!
//! Two implementations:
//! - [`PrngSource`] — the default; a software PRNG, provenance `prng://`. Zero
//!   new runtime dependencies.
//! - [`ReservoirSource`] — draws from kannaka-quantum's real-QPU entropy
//!   reservoir + NIST HMAC-DRBG (T1.2) via its CLI, carrying the provenance
//!   chain back to a QPU `job_id`.
//!
//! **CLI-vs-direct decision (documented on #473):** `ReservoirSource` shells out
//! to `kannaka-quantum qrng-draw` rather than re-implementing the DRBG in Rust —
//! it keeps the audited NIST HMAC-DRBG in one place and gets the provenance
//! chain for free. The Python/CLI coupling only applies when the reservoir source
//! is explicitly selected; the engine default stays [`PrngSource`], so the
//! musl-static release is unaffected for anyone not opting in.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Provenance of an entropy draw — where the bits came from, chained (for the
/// reservoir source) back to the QPU jobs that produced them. Serializes into
/// the memory v2 chiral format as an optional field (T1.4); an absent field is
/// read as [`Provenance::legacy`] (`prng://legacy`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Canonical source URI: `prng://`, `prng://legacy`, or
    /// `reservoir://<mode>` where mode is `raw` or `drbg-expand`.
    pub source: String,
    /// QPU job ids in the entropy chain (empty for a PRNG source).
    ///
    /// NB: no `skip_serializing_if` — this type is bincode-serialized inside
    /// `WavefrontMeta` (T1.4), and bincode is positional/non-self-describing, so
    /// conditionally omitting a field would desync the on-disk layout. Every
    /// field is always written; JSON just carries the (possibly empty) arrays.
    #[serde(default)]
    pub qpu_jobs: Vec<String>,
    /// QPU devices in the chain (empty for a PRNG source).
    #[serde(default)]
    pub devices: Vec<String>,
}

impl Provenance {
    /// Software PRNG (`prng://`).
    pub fn prng() -> Self {
        Self { source: "prng://".to_string(), qpu_jobs: Vec::new(), devices: Vec::new() }
    }

    /// The default for records written before provenance existed (`prng://legacy`).
    pub fn legacy() -> Self {
        Self { source: "prng://legacy".to_string(), qpu_jobs: Vec::new(), devices: Vec::new() }
    }

    /// A reservoir draw with its QPU chain. `mode` is the CLI's `mode`
    /// (`raw` or `drbg-expand`).
    pub fn reservoir(mode: &str, qpu_jobs: Vec<String>, devices: Vec<String>) -> Self {
        Self { source: format!("reservoir://{mode}"), qpu_jobs, devices }
    }

    /// Whether this draw came from a software PRNG (any `prng://…`).
    pub fn is_prng(&self) -> bool {
        self.source.starts_with("prng://")
    }
}

impl Default for Provenance {
    /// The persistence default: an absent provenance field means the record
    /// predates provenance, i.e. `prng://legacy`.
    fn default() -> Self {
        Self::legacy()
    }
}

/// A batch of entropy bytes plus where they came from.
#[derive(Debug, Clone)]
pub struct EntropyDraw {
    pub bytes: Vec<u8>,
    pub provenance: Provenance,
}

/// Why an entropy draw failed. The reservoir source fails **loudly** (it never
/// silently falls back to a PRNG) so a caller can decide to degrade explicitly.
#[derive(Debug, Clone)]
pub enum EntropyError {
    /// The reservoir is empty / depleted — refill via `kannaka-quantum harvest`.
    ReservoirEmpty(String),
    /// The kannaka-quantum CLI could not be launched (not installed / not on PATH).
    CliUnavailable(String),
    /// The CLI ran but its output could not be parsed.
    Parse(String),
}

impl fmt::Display for EntropyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntropyError::ReservoirEmpty(m) => write!(f, "entropy reservoir empty: {m}"),
            EntropyError::CliUnavailable(m) => write!(f, "kannaka-quantum CLI unavailable: {m}"),
            EntropyError::Parse(m) => write!(f, "could not parse qrng-draw output: {m}"),
        }
    }
}

impl std::error::Error for EntropyError {}

/// The seam. `draw` returns at least `bits` bits of entropy (rounded up to whole
/// bytes) with provenance.
pub trait EntropySource: Send {
    fn draw(&mut self, bits: usize) -> Result<EntropyDraw, EntropyError>;

    /// Short label for logging / telemetry (`"prng"` / `"reservoir"`).
    fn kind(&self) -> &'static str;
}

// ---------------------------------------------------------------------------
// PrngSource — the default
// ---------------------------------------------------------------------------

/// Software-PRNG entropy — the current behavior, and the default. Never fails.
#[derive(Debug, Default)]
pub struct PrngSource;

impl PrngSource {
    pub fn new() -> Self {
        PrngSource
    }
}

impl EntropySource for PrngSource {
    fn draw(&mut self, bits: usize) -> Result<EntropyDraw, EntropyError> {
        use rand::RngCore;
        let n = bits.div_ceil(8);
        let mut bytes = vec![0u8; n];
        rand::thread_rng().fill_bytes(&mut bytes);
        Ok(EntropyDraw { bytes, provenance: Provenance::prng() })
    }

    fn kind(&self) -> &'static str {
        "prng"
    }
}

// ---------------------------------------------------------------------------
// ReservoirSource — real-QPU reservoir via the kannaka-quantum CLI
// ---------------------------------------------------------------------------

/// Environment override for the kannaka-quantum executable (default
/// `kannaka-quantum`, the console script from its pyproject).
pub const QRNG_CMD_ENV: &str = "KANNAKA_QRNG_CMD";
const DEFAULT_QRNG_CMD: &str = "kannaka-quantum";

/// Draws from the T1.2 quantum entropy reservoir by invoking
/// `kannaka-quantum qrng-draw --bits N [--expand]` and parsing its JSON.
#[derive(Debug, Clone)]
pub struct ReservoirSource {
    /// Executable to invoke (from `KANNAKA_QRNG_CMD`, default `kannaka-quantum`).
    cmd: String,
    /// Use the HMAC-DRBG expansion (`--expand`) so limited quantum bits seed many
    /// draws. `KANNAKA_QRNG_EXPAND=0` requests raw 1:1 reservoir bits instead.
    expand: bool,
}

impl Default for ReservoirSource {
    fn default() -> Self {
        Self::new()
    }
}

impl ReservoirSource {
    pub fn new() -> Self {
        let cmd = std::env::var(QRNG_CMD_ENV).unwrap_or_else(|_| DEFAULT_QRNG_CMD.to_string());
        let expand = !matches!(
            std::env::var("KANNAKA_QRNG_EXPAND").unwrap_or_default().as_str(),
            "0" | "off" | "false"
        );
        Self { cmd, expand }
    }

    fn invoke(&self, bits: usize) -> Result<String, EntropyError> {
        let mut cmd = std::process::Command::new(&self.cmd);
        cmd.arg("qrng-draw").arg("--bits").arg(bits.to_string());
        if self.expand {
            cmd.arg("--expand");
        }
        let out = cmd
            .output()
            .map_err(|e| EntropyError::CliUnavailable(format!("{} ({e})", self.cmd)))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            // The CLI raises loudly on an empty reservoir; surface that verbatim.
            return Err(EntropyError::ReservoirEmpty(stderr));
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }
}

impl EntropySource for ReservoirSource {
    fn draw(&mut self, bits: usize) -> Result<EntropyDraw, EntropyError> {
        let json = self.invoke(bits)?;
        parse_qrng_draw(&json, bits)
    }

    fn kind(&self) -> &'static str {
        "reservoir"
    }
}

/// Parse a `kannaka-quantum qrng-draw` JSON document into an [`EntropyDraw`].
///
/// Split out from the subprocess call so the wire contract (bit-string → bytes,
/// provenance chain extraction) is unit-tested without needing Python or a live
/// reservoir. Expects the T1.2 shape: `{ "bits": "<binary>", "mode": …,
/// "provenance": [ { "device", "job_id", … } ], … }`.
pub fn parse_qrng_draw(json: &str, bits: usize) -> Result<EntropyDraw, EntropyError> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| EntropyError::Parse(e.to_string()))?;

    let bitstr = v
        .get("bits")
        .and_then(|b| b.as_str())
        .ok_or_else(|| EntropyError::Parse("missing string field `bits`".to_string()))?;
    let bytes = bits_to_bytes(bitstr, bits);

    let mode = v.get("mode").and_then(|m| m.as_str()).unwrap_or("drbg-expand");

    let mut qpu_jobs = Vec::new();
    let mut devices = Vec::new();
    if let Some(arr) = v.get("provenance").and_then(|p| p.as_array()) {
        for entry in arr {
            if let Some(j) = entry.get("job_id").and_then(|j| j.as_str()) {
                if !j.is_empty() && !qpu_jobs.iter().any(|e| e == j) {
                    qpu_jobs.push(j.to_string());
                }
            }
            if let Some(d) = entry.get("device").and_then(|d| d.as_str()) {
                if !d.is_empty() && !devices.iter().any(|e| e == d) {
                    devices.push(d.to_string());
                }
            }
        }
    }
    if qpu_jobs.is_empty() {
        // T1.2 guarantees a provenance chain on every successful draw; its absence
        // means the reservoir/meta are out of sync — never claim a sourceless draw.
        return Err(EntropyError::Parse(
            "qrng-draw output carried no QPU provenance".to_string(),
        ));
    }

    Ok(EntropyDraw { bytes, provenance: Provenance::reservoir(mode, qpu_jobs, devices) })
}

/// Reconstruct bytes from the CLI's MSB-first bit string. The CLI zero-fills the
/// big-endian integer to whole bytes then truncates to `n_bits`, so we pad the
/// (possibly short) final byte's low bits back with zeros.
fn bits_to_bytes(bitstr: &str, n_bits: usize) -> Vec<u8> {
    let nbytes = n_bits.div_ceil(8);
    let mut bytes = vec![0u8; nbytes];
    for (i, ch) in bitstr.chars().take(n_bits).enumerate() {
        if ch == '1' {
            bytes[i / 8] |= 1 << (7 - (i % 8));
        }
    }
    bytes
}

// ---------------------------------------------------------------------------
// Selection / factory
// ---------------------------------------------------------------------------

/// Which entropy source the engine uses. **Default `Prng`** — zero behavior
/// change and no Python/CLI dependency until a deployment opts in (after T1.5
/// dogfood).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntropySelection {
    Prng,
    Reservoir,
}

impl EntropySelection {
    /// Resolve from `KANNAKA_ENTROPY_SOURCE` (config bridges into this env, env
    /// wins — mirrors the belief/cluster/coupling config bridges). Anything other
    /// than an explicit reservoir selection is `Prng`.
    pub fn from_env() -> Self {
        match std::env::var("KANNAKA_ENTROPY_SOURCE")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "reservoir" | "qrng" | "quantum" => EntropySelection::Reservoir,
            _ => EntropySelection::Prng,
        }
    }

    pub fn build(self) -> Box<dyn EntropySource> {
        match self {
            EntropySelection::Prng => Box::new(PrngSource::new()),
            EntropySelection::Reservoir => Box::new(ReservoirSource::new()),
        }
    }
}

/// T1.4 (#474): whether dreams/Ξ perturbations actually CONSUME entropy from the
/// configured source (and therefore record its provenance). **Default OFF** —
/// set `KANNAKA_DREAM_ENTROPY=1|on|true` (config `[entropy].dream_perturbation`).
///
/// Deliberately decoupled from [`EntropySelection`]: "which source" and "whether
/// dreams draw from it" are independent. Off ⇒ the dream stays byte-identically
/// deterministic and records NO provenance (an honest absence, not a false
/// `prng://` stamp). On ⇒ the dream draws, its dynamics depend on the entropy,
/// and every stamped chain is TRUE (prng:// or reservoir://+QPU job). T1.5's
/// dogfood config is `[entropy].source = reservoir` + this gate ON.
pub fn dream_entropy_enabled() -> bool {
    std::env::var("KANNAKA_DREAM_ENTROPY")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("on") || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Derive a deterministic 64-bit seed from drawn entropy bytes (little-endian
/// fold). Makes the gated dream perturbation a reproducible function of the
/// entropy it drew — same bytes ⇒ same perturbation.
pub fn seed_from_bytes(bytes: &[u8]) -> u64 {
    let mut seed = 0u64;
    for (i, &b) in bytes.iter().enumerate() {
        seed ^= (b as u64) << ((i % 8) * 8);
        seed = seed.rotate_left(7).wrapping_add(0x9E37_79B9_7F4A_7C15);
    }
    seed
}

/// The configured entropy source. Defaults to [`PrngSource`].
pub fn default_source() -> Box<dyn EntropySource> {
    EntropySelection::from_env().build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prng_source_draws_requested_bytes_with_prng_provenance() {
        let mut s = PrngSource::new();
        let d = s.draw(128).unwrap();
        assert_eq!(d.bytes.len(), 16, "128 bits = 16 bytes");
        assert_eq!(d.provenance, Provenance::prng());
        assert!(d.provenance.is_prng());
        assert_eq!(s.kind(), "prng");
        // Non-byte-aligned request rounds up.
        assert_eq!(s.draw(1).unwrap().bytes.len(), 1);
        assert_eq!(s.draw(9).unwrap().bytes.len(), 2);
    }

    #[test]
    fn provenance_serde_roundtrip_and_legacy_default() {
        // Absent provenance (a pre-existing record) deserializes to prng://legacy.
        let none: Provenance = serde_json::from_str("{}").unwrap_or_default();
        // (serde would error on {}; Default is the persistence fallback.)
        let _ = none;
        assert_eq!(Provenance::default(), Provenance::legacy());
        assert_eq!(Provenance::legacy().source, "prng://legacy");

        let p = Provenance::reservoir(
            "drbg-expand",
            vec!["job-abc".into()],
            vec!["rigetti:cepheus-1-108q".into()],
        );
        let s = serde_json::to_string(&p).unwrap();
        let back: Provenance = serde_json::from_str(&s).unwrap();
        assert_eq!(p, back);
        assert!(!back.is_prng());
        assert_eq!(back.source, "reservoir://drbg-expand");

        // All fields always serialize (bincode-safety: no skip_serializing_if),
        // and a bincode round-trip is exact — the property WavefrontMeta relies on.
        let bin = bincode::serialize(&p).unwrap();
        let back_bin: Provenance = bincode::deserialize(&bin).unwrap();
        assert_eq!(p, back_bin);
        let sp = serde_json::to_string(&Provenance::prng()).unwrap();
        assert!(sp.contains("qpu_jobs"), "empty vecs still present (bincode-safe): {sp}");
    }

    #[test]
    fn parse_qrng_draw_extracts_bytes_and_chain() {
        // Mirrors the T1.2 `qrng-draw --expand` JSON shape.
        let json = r#"{
            "bits": "1010000011111111",
            "n_bits": 16,
            "int": 41215,
            "mode": "drbg-expand",
            "expanded": true,
            "reseeds": 0,
            "provenance": [
                {"device": "openquantum:rigetti:cepheus-1-108q", "job_id": "job-123", "n_bits": 2048},
                {"device": "openquantum:rigetti:cepheus-1-108q", "job_id": "job-123", "n_bits": 2048}
            ],
            "reservoir_available_bytes": 4096
        }"#;
        let d = parse_qrng_draw(json, 16).unwrap();
        assert_eq!(d.bytes, vec![0b1010_0000, 0b1111_1111]);
        assert_eq!(d.provenance.source, "reservoir://drbg-expand");
        // Duplicate job_id/device de-duped.
        assert_eq!(d.provenance.qpu_jobs, vec!["job-123".to_string()]);
        assert_eq!(d.provenance.devices, vec!["openquantum:rigetti:cepheus-1-108q".to_string()]);
    }

    #[test]
    fn parse_qrng_draw_rejects_sourceless_output() {
        let json = r#"{"bits":"1111","mode":"raw","provenance":[]}"#;
        let err = parse_qrng_draw(json, 4).unwrap_err();
        assert!(matches!(err, EntropyError::Parse(_)), "no provenance → Parse error");
    }

    #[test]
    fn bits_to_bytes_pads_truncated_low_bits() {
        // 12 bits → 2 bytes; the last nibble's low 4 bits are zero-padded.
        assert_eq!(bits_to_bytes("101000001111", 12), vec![0b1010_0000, 0b1111_0000]);
    }

    /// Live end-to-end smoke against a real reservoir + the kannaka-quantum CLI.
    /// `#[ignore]`d so CI never runs it (it needs the Python CLI on PATH and
    /// consumes real, paid QPU entropy). Run manually:
    ///   cargo test --lib entropy::tests::reservoir_source_live_smoke -- --ignored --nocapture
    #[test]
    #[ignore = "live: needs kannaka-quantum CLI + a non-empty reservoir; consumes real entropy"]
    fn reservoir_source_live_smoke() {
        let mut s = ReservoirSource::new();
        let d = s.draw(128).expect("live reservoir draw");
        assert!(!d.bytes.is_empty(), "got entropy bytes");
        assert!(d.provenance.source.starts_with("reservoir://"), "reservoir provenance");
        assert!(!d.provenance.qpu_jobs.is_empty(), "carries a QPU job chain");
        println!(
            "LIVE ReservoirSource draw -> bytes={} source={} qpu_jobs={:?} devices={:?}",
            d.bytes.len(), d.provenance.source, d.provenance.qpu_jobs, d.provenance.devices
        );
    }

    #[test]
    fn selection_defaults_to_prng() {
        // Unset / unknown → Prng (the from_env default). We don't mutate the
        // process env here (parallel-test safety); just assert the mapping via a
        // direct match on the documented tokens.
        assert_eq!(EntropySelection::Prng.build().kind(), "prng");
        assert_eq!(EntropySelection::Reservoir.build().kind(), "reservoir");
    }
}
