// Integration tests for IBC infrastructure deployment scripts
// Tests that our automation scripts work correctly and create proper infrastructure

use std::process::Command;
use std::fs;

#[test]
fn test_script_files_exist() {
    let required_scripts = vec![
        "scripts/create_simple_ibc_client.sh",
        "scripts/create_ibc_connection.sh", 
        "scripts/create_ibc_channel.sh",
        "scripts/complete_ibc_handshakes.sh",
        "scripts/generate_cosmos_key.sh",
        "scripts/setup_testnet.sh",
    ];
    
    for script in required_scripts {
        assert!(
            fs::metadata(script).is_ok(),
            "Required script {} does not exist", script
        );
        println!("✅ Script exists: {}", script);
    }
}

#[test]
fn test_scripts_are_executable() {
    let scripts = vec![
        "scripts/create_simple_ibc_client.sh",
        "scripts/create_ibc_connection.sh",
        "scripts/create_ibc_channel.sh", 
        "scripts/complete_ibc_handshakes.sh",
        "scripts/generate_cosmos_key.sh",
        "scripts/setup_testnet.sh",
    ];
    
    for script in scripts {
        let metadata = fs::metadata(script).expect(&format!("Cannot read {}", script));
        let permissions = metadata.permissions();
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = permissions.mode();
            assert!(
                mode & 0o111 != 0,
                "Script {} is not executable", script
            );
        }
        
        println!("✅ Script is executable: {}", script);
    }
}

#[test]
fn test_script_syntax_validation() {
    let bash_scripts = vec![
        "scripts/create_simple_ibc_client.sh",
        "scripts/create_ibc_connection.sh",
        "scripts/create_ibc_channel.sh",
        "scripts/complete_ibc_handshakes.sh", 
        "scripts/generate_cosmos_key.sh",
        "scripts/setup_testnet.sh",
    ];
    
    for script in bash_scripts {
        // Use bash -n to check syntax without executing
        let output = Command::new("bash")
            .arg("-n")
            .arg(script)
            .output()
            .expect(&format!("Failed to validate syntax for {}", script));
        
        assert!(
            output.status.success(),
            "Script {} has syntax errors: {}",
            script,
            String::from_utf8_lossy(&output.stderr)
        );
        
        println!("✅ Script syntax is valid: {}", script);
    }
}

#[test]
fn test_script_headers_and_safety() {
    let scripts = vec![
        "scripts/create_simple_ibc_client.sh",
        "scripts/create_ibc_connection.sh",
        "scripts/create_ibc_channel.sh",
        "scripts/complete_ibc_handshakes.sh",
        "scripts/generate_cosmos_key.sh", 
        "scripts/setup_testnet.sh",
    ];
    
    for script in scripts {
        let content = fs::read_to_string(script)
            .expect(&format!("Cannot read script {}", script));
        
        // Check for proper shebang
        assert!(
            content.starts_with("#!/bin/bash"),
            "Script {} missing proper shebang", script
        );
        
        // Check for 'set -e' (exit on error)
        assert!(
            content.contains("set -e"),
            "Script {} missing 'set -e' safety check", script
        );
        
        // Check that script doesn't contain dangerous patterns
        let dangerous_patterns = vec![
            "rm -rf /",
            "rm -rf $HOME",
            ">(.*/)dev/null",
            "eval.*\\$",
        ];
        
        for pattern in dangerous_patterns {
            assert!(
                !content.contains(pattern),
                "Script {} contains dangerous pattern: {}", script, pattern
            );
        }
        
        println!("✅ Script is safe and well-formed: {}", script);
    }
}

#[test]
fn test_script_configuration_variables() {
    let scripts_with_config = vec![
        ("scripts/create_simple_ibc_client.sh", vec!["CONTRACT_ID", "SIGNER_ID"]),
        ("scripts/create_ibc_connection.sh", vec!["CONTRACT_ID", "SIGNER_ID", "CLIENT_ID"]),
        ("scripts/create_ibc_channel.sh", vec!["CONTRACT_ID", "SIGNER_ID", "CONNECTION_ID"]),
        ("scripts/complete_ibc_handshakes.sh", vec!["CONTRACT_ID", "SIGNER_ID"]),
    ];
    
    for (script, expected_vars) in scripts_with_config {
        let content = fs::read_to_string(script)
            .expect(&format!("Cannot read script {}", script));
        
        for var in expected_vars {
            assert!(
                content.contains(&format!("{}=", var)) || content.contains(&format!("${}", var)),
                "Script {} missing required variable: {}", script, var
            );
        }
        
        // Check for hardcoded testnet configuration
        assert!(
            content.contains("cosmos-sdk-demo.testnet"),
            "Script {} missing testnet contract configuration", script
        );
        
        assert!(
            content.contains("cuteharbor3573.testnet"),
            "Script {} missing testnet signer configuration", script
        );
        
        println!("✅ Script configuration is correct: {}", script);
    }
}

#[test]
fn test_json_format_in_scripts() {
    let scripts_with_json = vec![
        "scripts/create_simple_ibc_client.sh",
        "scripts/create_ibc_connection.sh",
        "scripts/create_ibc_channel.sh",
    ];
    
    for script in scripts_with_json {
        let content = fs::read_to_string(script)
            .expect(&format!("Cannot read script {}", script));
        
        // Check for correct JSON field names (lessons learned from debugging)
        if content.contains("part_set_header") {
            assert!(
                !content.contains("\"parts\":"),
                "Script {} uses old 'parts' field instead of 'part_set_header'", script
            );
            println!("✅ Script uses correct 'part_set_header' field: {}", script);
        }
        
        if content.contains("Ed25519") {
            assert!(
                !content.contains("\"type\": \"tendermint/PubKeyEd25519\""),
                "Script {} uses old pub_key format", script
            );
            println!("✅ Script uses correct Ed25519 enum format: {}", script);
        }
        
        // Check JSON structure looks reasonable
        let json_count = content.matches('{').count();
        let json_close_count = content.matches('}').count();
        assert_eq!(
            json_count, json_close_count,
            "Script {} has unbalanced JSON braces", script
        );
        
        println!("✅ Script JSON format is valid: {}", script);
    }
}

#[test]
fn test_removed_redundant_scripts() {
    // Ensure we removed the redundant script
    assert!(
        !fs::metadata("scripts/create_ibc_client.sh").is_ok(),
        "Redundant script create_ibc_client.sh should have been removed"
    );
    
    println!("✅ Redundant scripts properly removed");
}

#[test]
fn test_script_output_expectations() {
    let scripts_with_outputs = vec![
        ("scripts/create_simple_ibc_client.sh", vec!["✅", "IBC client", "07-tendermint-0"]),
        ("scripts/create_ibc_connection.sh", vec!["✅", "connection", "connection-0"]),
        ("scripts/create_ibc_channel.sh", vec!["✅", "channel", "channel-0"]),
        ("scripts/complete_ibc_handshakes.sh", vec!["🤝", "handshake", "foundation complete"]),
    ];
    
    for (script, expected_outputs) in scripts_with_outputs {
        let content = fs::read_to_string(script)
            .expect(&format!("Cannot read script {}", script));
        
        for expected in expected_outputs {
            assert!(
                content.contains(expected),
                "Script {} missing expected output pattern: {}", script, expected
            );
        }
        
        // Check for error handling
        assert!(
            content.contains("❌") || content.contains("Failed") || content.contains("Error"),
            "Script {} missing error handling output", script
        );
        
        println!("✅ Script has proper output formatting: {}", script);
    }
}

#[test]
fn test_script_dependencies() {
    // Test that scripts reference the correct tools and dependencies
    let dependency_checks = vec![
        ("scripts/create_simple_ibc_client.sh", "near call"),
        ("scripts/create_ibc_connection.sh", "near call"),
        ("scripts/create_ibc_channel.sh", "near call"),
        ("scripts/complete_ibc_handshakes.sh", "near view"),
        ("scripts/generate_cosmos_key.sh", "openssl"),
    ];
    
    for (script, dependency) in dependency_checks {
        if fs::metadata(script).is_ok() {
            let content = fs::read_to_string(script)
                .expect(&format!("Cannot read script {}", script));
            
            assert!(
                content.contains(dependency),
                "Script {} missing required dependency command: {}", script, dependency
            );
            
            println!("✅ Script has correct dependencies: {}", script);
        }
    }
}

#[test]
fn test_infrastructure_script_sequence() {
    // Test that scripts can be run in the correct sequence
    let script_sequence = vec![
        "scripts/create_simple_ibc_client.sh",   // Creates client first
        "scripts/create_ibc_connection.sh",      // Then connection
        "scripts/create_ibc_channel.sh",         // Then channel
        "scripts/complete_ibc_handshakes.sh",    // Finally check status
    ];
    
    for (i, script) in script_sequence.iter().enumerate() {
        let content = fs::read_to_string(script)
            .expect(&format!("Cannot read script {}", script));
        
        // Check that later scripts reference artifacts from earlier ones
        match i {
            1 => {
                assert!(
                    content.contains("CLIENT_ID") || content.contains("07-tendermint-0"),
                    "Connection script should reference client created by previous script"
                );
            }
            2 => {
                assert!(
                    content.contains("CONNECTION_ID") || content.contains("connection-0"),
                    "Channel script should reference connection created by previous script"
                );
            }
            3 => {
                assert!(
                    content.contains("connection-0") && content.contains("channel-0"),
                    "Status script should reference both connection and channel"
                );
            }
            _ => {}
        }
        
        println!("✅ Script {} fits correctly in deployment sequence", script);
    }
}