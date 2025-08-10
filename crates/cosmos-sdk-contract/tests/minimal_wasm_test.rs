/// Test the x/wasm module using a minimal contract to isolate the initialization issue

use anyhow::Result;

#[tokio::test]  
async fn test_full_contract_size_analysis() -> Result<()> {
    println!("🔍 Analyzing full contract WASM size and structure...");
    
    let wasm_path = "./target/near/cosmos_sdk_near.wasm";
    let wasm_data = std::fs::read(wasm_path)?;
    
    println!("📊 WASM Analysis:");
    println!("  File size: {} bytes ({:.2} MB)", wasm_data.len(), wasm_data.len() as f64 / 1_000_000.0);
    
    // Check WASM magic number
    if wasm_data.len() >= 4 {
        let magic = &wasm_data[0..4];
        if magic == b"\x00asm" {
            println!("✅ Valid WASM magic number");
        } else {
            println!("❌ Invalid WASM magic number: {:?}", magic);
        }
    }
    
    // Try to get more detailed info about the WASM structure
    let mut import_count = 0;
    let mut export_count = 0;
    let mut function_count = 0;
    
    for payload in wasmparser::Parser::new(0).parse_all(&wasm_data) {
        match payload? {
            wasmparser::Payload::ImportSection(reader) => {
                import_count = reader.count();
            }
            wasmparser::Payload::ExportSection(reader) => {
                export_count = reader.count();
            }
            wasmparser::Payload::FunctionSection(reader) => {
                function_count = reader.count();
            }
            _ => {}
        }
    }
    
    println!("  Imports: {}", import_count);
    println!("  Exports: {}", export_count);
    println!("  Functions: {}", function_count);
    
    Ok(())
}