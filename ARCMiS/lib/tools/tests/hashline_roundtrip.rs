//! Round-trip test: read a file, apply a SWAP patch, re-read, assert the tag
//! changed and the edit landed.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use oxi_hashline::InMemorySnapshotStore;
use rig::tool::Tool;
use tools::edit::EditArgs;
use tools::read::ReadArgs;
use tools::write::WriteArgs;
use tools::Edit;
use tools::Read;
use tools::Write;

/// Root of a throwaway sandbox directory, unique per test name.
fn sandbox(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("arcmis-hashline-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[tokio::test]
async fn read_write_edit_roundtrip_changes_tag() {
    let root = sandbox("roundtrip");
    let store: Arc<dyn oxi_hashline::SnapshotStore> = Arc::new(InMemorySnapshotStore::new());

    let write = Write {
        root: root.clone(),
        snapshots: store.clone(),
    };
    let result = write
        .call(
            &mut rig::tool::ToolContext::new(),
            WriteArgs {
                path: "src/lib.rs".to_owned(),
                content: "fn one() -> u32 {\n    1\n}\n".to_owned(),
            },
        )
        .await
        .unwrap();
    let text = result.as_text().unwrap_or_default().to_owned();
    let header = text.lines().next().unwrap();
    assert!(header.starts_with("[") && header.contains("#") && header.trim_end().ends_with("]"), "header was {header}");
    let first_tag = header.trim_end_matches(']').rsplit('#').next().unwrap().to_owned();

    let read = Read {
        root: root.clone(),
        snapshots: store.clone(),
    };
    let result = read
        .call(
            &mut rig::tool::ToolContext::new(),
            ReadArgs {
                path: "src/lib.rs".to_owned(),
                offset: None,
                limit: None,
            },
        )
        .await
        .unwrap();
    let text = result.as_text().unwrap_or_default().to_owned();
    let header = text.lines().next().unwrap();
    assert!(header.starts_with("[") && header.contains("#") && header.ends_with("]"), "header was {header}");
    // Body lines are `N:content`.
    assert!(text.contains("1:fn one() -> u32 {"), "body was {text}");

    // SWAP line 2 through the edit tool.
    let edit = Edit::new(root.clone(), store.clone());
    let patch = format!("[src/lib.rs#{first_tag}]\nSWAP 2.=2:\n+    2\n");
    let result = edit
        .call(
            &mut rig::tool::ToolContext::new(),
            EditArgs {
                patch,
            },
        )
        .await;
    if let Err(error) = &result {
        panic!("edit failed: {error}");
    }
    let result = result.unwrap();
    let text = result.as_text().unwrap_or_default().to_owned();
    let header = text.lines().next().unwrap();
    let new_tag = header.trim_end_matches(']').rsplit('#').next().unwrap().to_owned();
    assert_ne!(first_tag, new_tag, "tag must change after an edit");

    let on_disk = fs::read_to_string(root.join("src/lib.rs")).unwrap();
    assert_eq!(on_disk, "fn one() -> u32 {\n    2\n}\n");

    // Re-read shows the new tag.
    let result = read
        .call(
            &mut rig::tool::ToolContext::new(),
            ReadArgs {
                path: "src/lib.rs".to_owned(),
                offset: None,
                limit: None,
            },
        )
        .await
        .unwrap();
    let text = result.as_text().unwrap_or_default().to_owned();
    assert!(text.starts_with('[') && text.contains(&format!("#{new_tag}]")), "was {text}");

    let _ = fs::remove_dir_all(&root);
}

#[tokio::test]
async fn stale_tag_is_rejected() {
    let root = sandbox("stale");
    let store: Arc<dyn oxi_hashline::SnapshotStore> = Arc::new(InMemorySnapshotStore::new());

    let write = Write {
        root: root.clone(),
        snapshots: store.clone(),
    };
    write
        .call(
            &mut rig::tool::ToolContext::new(),
            WriteArgs {
                path: "a.txt".to_owned(),
                content: "line one\nline two\n".to_owned(),
            },
        )
        .await
        .unwrap();

    // Patch with a tag the session never minted.
    let edit = Edit::new(root.clone(), store.clone());
    let result = edit
        .call(
            &mut rig::tool::ToolContext::new(),
            EditArgs {
                patch: "[a.txt#ZZZZ]\nDEL 1.=1\n".to_owned(),
            },
        )
        .await;
    assert!(result.is_err(), "stale tag must be rejected");
    let message = result.err().unwrap().to_string();
    assert!(
        message.contains("ZZZZ") || message.to_lowercase().contains("hash"),
        "error should name the stale tag: {message}"
    );

    let _ = fs::remove_dir_all(&root);
}
