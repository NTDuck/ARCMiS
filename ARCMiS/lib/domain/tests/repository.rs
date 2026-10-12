//! Integration tests for the repository module.

use std::path::PathBuf;

use mas_domain::RepositoryMetadata;

#[test]
fn metadata_constructor_stores_every_field() {
    let metadata = RepositoryMetadata::new(PathBuf::from("/repo"), vec!["rust".to_owned(), "toml".to_owned()], 42);
    assert_eq!(metadata.root, PathBuf::from("/repo"));
    assert_eq!(metadata.languages, vec!["rust", "toml"]);
    assert_eq!(metadata.file_count, 42);
}

#[test]
fn metadata_equality_holds() {
    let first = RepositoryMetadata::new(PathBuf::from("/repo"), vec!["rust".to_owned()], 10);
    let second = RepositoryMetadata::new(PathBuf::from("/repo"), vec!["rust".to_owned()], 10);
    assert_eq!(first, second);
}
