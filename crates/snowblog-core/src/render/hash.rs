pub fn input_hash(source: &str, asset_manifest: &[(String, String)]) -> String {
    let mut sorted: Vec<&(String, String)> = asset_manifest.iter().collect();
    sorted.sort();
    let mut hasher = blake3::Hasher::new();
    hasher.update(source.as_bytes());
    for (path, content_hash) in sorted {
        hasher.update(&[0]);
        hasher.update(path.as_bytes());
        hasher.update(&[0]);
        hasher.update(content_hash.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(p, h)| (p.to_string(), h.to_string()))
            .collect()
    }

    #[test]
    fn changes_with_source() {
        let assets = manifest(&[("a.png", "h1")]);
        assert_ne!(input_hash("= A", &assets), input_hash("= B", &assets));
    }

    #[test]
    fn changes_with_asset_hash() {
        assert_ne!(
            input_hash("= A", &manifest(&[("a.png", "h1")])),
            input_hash("= A", &manifest(&[("a.png", "h2")]))
        );
    }

    #[test]
    fn independent_of_manifest_order() {
        assert_eq!(
            input_hash("= A", &manifest(&[("a.png", "h1"), ("b.png", "h2")])),
            input_hash("= A", &manifest(&[("b.png", "h2"), ("a.png", "h1")]))
        );
    }

    #[test]
    fn path_and_hash_boundaries_are_unambiguous() {
        assert_ne!(
            input_hash("= A", &manifest(&[("ab", "c")])),
            input_hash("= A", &manifest(&[("a", "bc")]))
        );
    }
}
