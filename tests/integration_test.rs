use nothing::file_entry::FileEntry;
use nothing::index::FileIndex;
use nothing::persistence;
use nothing::search::SearchEngine;
use chrono::Utc;

#[test]
fn test_search_functionality() {
    // Load the test index
    let cache_path = persistence::get_index_path('C').expect("Failed to get index path");
    let index = persistence::load_index(&cache_path).expect("Failed to load test index");
    let mut search_engine = SearchEngine::new();

    println!("Loaded index with {} files", index.file_count());
    assert!(index.file_count() > 0, "Index should have files");

    // Test basic search
    let results = search_engine.search(&index, "README", 10);
    println!("Search 'README': {} results", results.len());
    assert!(results.len() > 0, "Should find README file");
    assert!(results[0].entry.name.contains("README"), "Top result should contain README");

    // Test case insensitive search
    let results_lower = search_engine.search(&index, "readme", 10);
    println!("Search 'readme': {} results", results_lower.len());
    assert_eq!(results.len(), results_lower.len(), "Search should be case insensitive");

    // Test extension search
    let results = search_engine.search(&index, ".rs", 20);
    println!("Search '.rs': {} results", results.len());
    let rs_files: Vec<_> = results.iter().filter(|r| r.entry.name.ends_with(".rs")).collect();
    println!("  .rs files found: {}", rs_files.len());
    assert!(rs_files.len() > 0, "Should find .rs files");

    // Test partial match
    let results = search_engine.search(&index, "main", 10);
    println!("Search 'main': {} results", results.len());
    assert!(results.len() > 0, "Should find files with 'main'");

    // Test directory search
    let results = search_engine.search(&index, "src", 10);
    println!("Search 'src': {} results", results.len());
    assert!(results.len() > 0, "Should find src directory");

    // Test fuzzy matching
    let results = search_engine.search(&index, "carg", 10);
    println!("Search 'carg' (fuzzy): {} results", results.len());
    assert!(results.iter().any(|r| r.entry.name.contains("Cargo")), "Fuzzy search should find Cargo");

    // Test no results
    let results = search_engine.search(&index, "xyznonexistent123", 10);
    println!("Search 'xyznonexistent123': {} results", results.len());
    assert_eq!(results.len(), 0, "Should return no results for nonexistent file");

    println!("\n✅ All search tests passed!");
}

#[test]
fn test_index_persistence() {
    // Create a temporary index
    let mut index = FileIndex::new();

    let now = Utc::now();
    let entry = FileEntry::new(
        "test_file.txt".to_string(),
        "C:\\Test\\test_file.txt".to_string(),
        false,
        12345,
        0,
        1024,
        Some(now),
        Some(now),
        Some(now),
    );

    index.add_entry(entry);

    assert_eq!(index.file_count(), 1, "Index should have 1 file");

    // Test serialization/deserialization
    let temp_path = std::env::temp_dir().join("nothing_test_index.bin");
    persistence::save_index(&index, temp_path.to_str().unwrap()).expect("Failed to save index");

    let loaded_index = persistence::load_index(temp_path.to_str().unwrap()).expect("Failed to load index");
    assert_eq!(loaded_index.file_count(), 1, "Loaded index should have 1 file");

    let mut search_engine = SearchEngine::new();
    let results = search_engine.search(&loaded_index, "test_file", 10);
    assert_eq!(results.len(), 1, "Should find the test file");
    assert_eq!(results[0].entry.name, "test_file.txt", "File name should match");

    // Cleanup
    let _ = std::fs::remove_file(temp_path);

    println!("✅ Index persistence test passed!");
}
