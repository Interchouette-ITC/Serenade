//! Verifies the GitHub build-provenance attestation of a downloaded Trunk archive.
//!
//! The fork's release workflow attests every archive with
//! `actions/attest-build-provenance`, signed through the Sigstore public-good
//! instance. The checksum published next to an archive only detects corruption;
//! this attestation proves which workflow built it. Verification uses Sigstore's
//! embedded production trust root: certificate chain, SCT, Rekor inclusion,
//! DSSE signature, the in-toto subject digest, and the exact signer workflow and
//! OIDC issuer.

use std::time::Duration;

use anyhow::{Context, bail};
use serde::Deserialize;
use sigstore_trust_root::{SIGSTORE_PRODUCTION_TRUSTED_ROOT, TrustedRoot};
use sigstore_types::{Bundle, Sha256Hash, SignatureContent};
use sigstore_verify::{IdentityMatcher, VerificationPolicy, Verifier};

/// GitHub attestations API of the Trunk fork; `/sha256:<digest>` is appended.
pub const ATTESTATIONS_URL: &str =
    "https://api.github.com/repos/Interchouette-ITC/trunk/attestations";
/// Certificate identity of the fork's release workflow on `main`.
pub const SIGNER_WORKFLOW: &str =
    "https://github.com/Interchouette-ITC/trunk/.github/workflows/release.yaml@refs/heads/main";
const OIDC_ISSUER: &str = "https://token.actions.githubusercontent.com";
const IN_TOTO_PAYLOAD_TYPE: &str = "application/vnd.in-toto+json";
const SLSA_PROVENANCE_V1: &str = "https://slsa.dev/provenance/v1";
const API_TIMEOUT: Duration = Duration::from_secs(30);

/// Facts established by a verified attestation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    /// Git commit the attested build was produced from.
    pub commit: String,
}

#[derive(Deserialize)]
struct Listing {
    #[serde(default)]
    attestations: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    bundle: Option<serde_json::Value>,
    bundle_url: Option<String>,
}

#[derive(Deserialize)]
struct Statement {
    #[serde(rename = "predicateType")]
    predicate_type: String,
    #[serde(default)]
    predicate: serde_json::Value,
}

/// Fetches the attestations GitHub holds for `sha256_hex` and returns the first
/// one that verifies as build provenance signed by `signer_workflow`.
///
/// `GITHUB_TOKEN` or `GH_TOKEN` is sent to the GitHub API when set, to lift the
/// anonymous rate limit; it is never sent to the bundle storage URLs.
///
/// # Errors
///
/// Returns an error when the API cannot be reached, no attestation exists, or
/// none verifies against the expected signer.
pub fn verify_archive(
    api_url: &str,
    sha256_hex: &str,
    signer_workflow: &str,
) -> anyhow::Result<Provenance> {
    let digest = Sha256Hash::from_hex(sha256_hex)
        .map_err(|error| anyhow::anyhow!("invalid SHA-256 digest {sha256_hex:?}: {error}"))?;
    let url = format!("{}/sha256:{sha256_hex}", api_url.trim_end_matches('/'));
    let mut headers = vec![
        ("accept", "application/vnd.github+json".to_owned()),
        ("x-github-api-version", "2022-11-28".to_owned()),
    ];
    if let Some(token) = ["GITHUB_TOKEN", "GH_TOKEN"]
        .into_iter()
        .find_map(|name| std::env::var(name).ok().filter(|token| !token.is_empty()))
    {
        headers.push(("authorization", format!("Bearer {token}")));
    }

    let (status, body) = super::http_fetch(&url, API_TIMEOUT, headers)?;
    if status == 404 {
        bail!("no build provenance attestation is published for sha256 {sha256_hex}");
    }
    if !(200..300).contains(&status) {
        let hint = if matches!(status, 401 | 403 | 429) {
            " (rate limited? set GITHUB_TOKEN)"
        } else {
            ""
        };
        bail!("GitHub attestations API returned HTTP {status} for {url}{hint}");
    }
    let listing: Listing = serde_json::from_slice(&body)
        .with_context(|| format!("malformed attestations response from {url}"))?;

    let mut failures = Vec::new();
    for entry in listing.attestations {
        let outcome =
            entry_bundle(&entry).and_then(|json| verify_bundle(&json, digest, signer_workflow));
        match outcome {
            Ok(provenance) => return Ok(provenance),
            Err(error) => failures.push(format!("{error:#}")),
        }
    }
    if failures.is_empty() {
        bail!("no build provenance attestation is published for sha256 {sha256_hex}");
    }
    bail!(
        "no attestation for sha256 {sha256_hex} verifies as build provenance from {signer_workflow}: {}",
        failures.join("; ")
    )
}

fn entry_bundle(entry: &Entry) -> anyhow::Result<String> {
    if let Some(bundle) = entry.bundle.as_ref().filter(|bundle| !bundle.is_null()) {
        return Ok(bundle.to_string());
    }
    let url = entry
        .bundle_url
        .as_deref()
        .context("attestation has neither a bundle nor a bundle URL")?;
    let (status, body) = super::http_fetch(url, API_TIMEOUT, Vec::new())?;
    if !(200..300).contains(&status) {
        bail!("attestation bundle download returned HTTP {status}");
    }
    decode_bundle(&body)
}

/// GitHub serves stored bundles Snappy-compressed (`.json.sn`); plain JSON is
/// accepted as well.
fn decode_bundle(bytes: &[u8]) -> anyhow::Result<String> {
    let json = if bytes.first() == Some(&b'{') {
        bytes.to_vec()
    } else {
        snap::raw::Decoder::new()
            .decompress_vec(bytes)
            .context("failed to decompress the attestation bundle")?
    };
    String::from_utf8(json).context("attestation bundle is not UTF-8")
}

/// Verifies one Sigstore bundle as SLSA build provenance for `digest`, signed
/// by `signer_workflow` through GitHub Actions OIDC.
fn verify_bundle(
    bundle_json: &str,
    digest: Sha256Hash,
    signer_workflow: &str,
) -> anyhow::Result<Provenance> {
    let bundle = Bundle::from_json(bundle_json)
        .map_err(|error| anyhow::anyhow!("malformed Sigstore bundle: {error}"))?;
    let SignatureContent::DsseEnvelope(envelope) = &bundle.content else {
        bail!("attestation is not a DSSE envelope");
    };
    if envelope.payload_type != IN_TOTO_PAYLOAD_TYPE {
        bail!(
            "unexpected attestation payload type {}",
            envelope.payload_type
        );
    }
    let statement: Statement = serde_json::from_slice(envelope.payload.as_bytes())
        .context("malformed in-toto statement")?;
    if statement.predicate_type != SLSA_PROVENANCE_V1 {
        bail!(
            "attestation is {}, not SLSA build provenance",
            statement.predicate_type
        );
    }

    let trusted_root = TrustedRoot::from_json(SIGSTORE_PRODUCTION_TRUSTED_ROOT)
        .map_err(|error| anyhow::anyhow!("invalid embedded Sigstore trust root: {error}"))?;
    let policy = VerificationPolicy::new(
        IdentityMatcher::Uri(signer_workflow.to_owned()),
        OIDC_ISSUER,
    );
    Verifier::new(&trusted_root)
        .and_then(|verifier| verifier.verify(digest, &bundle, &policy))
        .map_err(|error| anyhow::anyhow!("Sigstore verification failed: {error}"))?;

    let commit = source_commit(&statement.predicate, signer_workflow)?;
    Ok(Provenance { commit })
}

/// Returns the Git commit of the signer's own repository among the provenance's
/// resolved dependencies, ignoring any other Git input.
fn source_commit(predicate: &serde_json::Value, signer_workflow: &str) -> anyhow::Result<String> {
    let repository = signer_workflow
        .split_once("/.github/workflows/")
        .map(|(repository, _)| repository)
        .context("signer workflow is not a GitHub Actions workflow URI")?;
    let source_prefix = format!("git+{repository}@");
    predicate["buildDefinition"]["resolvedDependencies"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|dependency| {
            dependency["uri"]
                .as_str()
                .is_some_and(|uri| uri.starts_with(&source_prefix))
        })
        .find_map(|dependency| dependency["digest"]["gitCommit"].as_str())
        .filter(|commit| commit.len() == 40 && commit.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .map(str::to_ascii_lowercase)
        .with_context(|| format!("provenance does not name a {repository} source commit"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build provenance GitHub published for `gh_2.104.0_linux_amd64.tar.gz`
    /// (cli/cli), as served by the attestations API: Snappy-compressed JSON.
    const GH_BUNDLE: &[u8] =
        include_bytes!("../../tests/fixtures/gh-cli-provenance.sigstore.json.sn");
    const GH_DIGEST: &str = "98cb101ad87dbbd3f6b2f2bfcbd43197ccd07643140121d8a9cfaa91eadbc24e";
    const GH_SIGNER: &str =
        "https://github.com/cli/cli/.github/workflows/deployment.yml@refs/heads/trunk";

    fn digest(hex: &str) -> Sha256Hash {
        Sha256Hash::from_hex(hex).expect("digest")
    }

    #[test]
    fn verifies_real_github_build_provenance() {
        let json = decode_bundle(GH_BUNDLE).expect("decode snappy bundle");
        let provenance = verify_bundle(&json, digest(GH_DIGEST), GH_SIGNER).expect("verifies");
        assert_eq!(provenance.commit.len(), 40);
        assert_eq!(decode_bundle(json.as_bytes()).expect("plain json"), json);
    }

    #[test]
    fn rejects_another_signer_or_artifact() {
        let json = decode_bundle(GH_BUNDLE).expect("decode snappy bundle");
        let error = verify_bundle(&json, digest(GH_DIGEST), SIGNER_WORKFLOW)
            .expect_err("cli/cli is not the Trunk fork");
        assert!(format!("{error:#}").contains("Sigstore verification failed"));

        let error = verify_bundle(&json, digest(&"0".repeat(64)), GH_SIGNER)
            .expect_err("digest is not an attested subject");
        assert!(format!("{error:#}").contains("Sigstore verification failed"));
    }

    #[test]
    fn rejects_tampered_bundles() {
        let json = decode_bundle(GH_BUNDLE).expect("decode snappy bundle");
        let mut bundle: serde_json::Value = serde_json::from_str(&json).expect("json");
        let signature = bundle["dsseEnvelope"]["signatures"][0]["sig"]
            .as_str()
            .expect("signature")
            .to_owned();
        let flipped = if signature.starts_with('A') { "B" } else { "A" };
        bundle["dsseEnvelope"]["signatures"][0]["sig"] =
            format!("{flipped}{}", &signature[1..]).into();
        assert!(verify_bundle(&bundle.to_string(), digest(GH_DIGEST), GH_SIGNER).is_err());
        assert!(decode_bundle(b"\xff\xff not snappy").is_err());
    }

    #[test]
    fn source_commit_comes_only_from_the_signer_repository() {
        let other = "1".repeat(40);
        let fork = "2".repeat(40);
        let predicate = serde_json::json!({"buildDefinition": {"resolvedDependencies": [
            {"uri": "git+https://github.com/someone/else@refs/heads/main", "digest": {"gitCommit": other}},
            {"uri": "git+https://github.com/Interchouette-ITC/trunk-evil@refs/heads/main", "digest": {"gitCommit": other}},
            {"uri": "git+https://github.com/Interchouette-ITC/trunk@refs/heads/main", "digest": {"gitCommit": fork}},
        ]}});
        assert_eq!(
            source_commit(&predicate, SIGNER_WORKFLOW).expect("fork commit"),
            fork
        );

        let foreign = serde_json::json!({"buildDefinition": {"resolvedDependencies": [
            {"uri": "git+https://github.com/someone/else@refs/heads/main", "digest": {"gitCommit": other}},
        ]}});
        let error = source_commit(&foreign, SIGNER_WORKFLOW).expect_err("no fork commit");
        assert!(format!("{error:#}").contains("Interchouette-ITC/trunk source commit"));
    }

    #[cfg(unix)]
    #[test]
    fn fetches_bundles_through_the_attestations_api() {
        use std::{
            io::{BufRead, BufReader, Write},
            net::TcpListener,
        };

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let base = format!("http://{}", listener.local_addr().expect("address"));
        let listing = format!(
            r#"{{"attestations":[{{"bundle":null,"bundle_url":"{base}/blob/bundle.json.sn"}}]}}"#
        );
        let listing_path = format!("/attestations/sha256:{GH_DIGEST}");
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().expect("clone"));
                let mut request = String::new();
                reader.read_line(&mut request).expect("request line");
                let mut header = String::new();
                while reader.read_line(&mut header).is_ok_and(|read| read > 2) {
                    header.clear();
                }
                let path = request.split_whitespace().nth(1).unwrap_or("/");
                let (status, body): (&str, &[u8]) = if path == listing_path {
                    ("200 OK", listing.as_bytes())
                } else if path == "/blob/bundle.json.sn" {
                    ("200 OK", GH_BUNDLE)
                } else {
                    ("404 Not Found", b"")
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(body);
            }
        });

        let api = format!("{base}/attestations");
        let provenance = verify_archive(&api, GH_DIGEST, GH_SIGNER).expect("verified");
        assert_eq!(provenance.commit.len(), 40);

        let error = verify_archive(&api, GH_DIGEST, SIGNER_WORKFLOW).expect_err("wrong signer");
        assert!(format!("{error:#}").contains("verifies as build provenance"));

        let error = verify_archive(&api, &"1".repeat(64), GH_SIGNER).expect_err("no attestation");
        assert!(format!("{error:#}").contains("no build provenance attestation"));
    }

    #[test]
    #[ignore = "queries the live GitHub attestations API; run with --ignored"]
    fn verifies_through_the_live_github_api() {
        let provenance = verify_archive(
            "https://api.github.com/repos/cli/cli/attestations",
            GH_DIGEST,
            GH_SIGNER,
        )
        .expect("live attestation verifies");
        assert_eq!(provenance.commit.len(), 40);
    }
}
