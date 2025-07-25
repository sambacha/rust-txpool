use regex::Regex;
use serde_json::{Value, json};
use std::fs::File;
use std::io::{self, Read};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let json_value = parse_debug_format(&input)?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();
    let filename = format!("txpool_{}.json", timestamp);

    let mut file = File::create(&filename)?;
    serde_json::to_writer_pretty(&mut file, &json_value)?;

    println!("Converted output saved to {}", filename);
    Ok(())
}

fn parse_debug_format(input: &str) -> Result<Value, Box<dyn std::error::Error>> {
    // Check which format we're dealing with
    if input.contains("TxpoolContent") {
        parse_txpool_content(input)
    } else if input.contains("TxpoolInspect") {
        parse_txpool_inspect(input)
    } else {
        Err("Unknown debug format".into())
    }
}

fn parse_txpool_inspect(input: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let mut root = json!({ "pending": {} });
    let pending = root["pending"].as_object_mut().unwrap();

    // Regex to capture address and its transactions
    let addr_re = Regex::new(r"(\w{40}): {")?;
    let mut current_addr = None;
    let mut current_nonce = None;

    for line in input.lines() {
        let trimmed = line.trim();

        // Skip empty lines and struct names
        if trimmed.is_empty() || trimmed.starts_with("TxpoolInspect") {
            continue;
        }

        // Capture Ethereum address
        if let Some(caps) = addr_re.captures(trimmed) {
            current_addr = Some(format!("0x{}", &caps[1]));
            continue;
        }

        // Capture nonce
        if let Some(nonce) = trimmed.strip_suffix(": TxpoolInspectSummary {") {
            if let Some(addr) = &current_addr {
                let nonce = nonce.trim_matches('"');
                current_nonce = Some(nonce.to_string());
                pending
                    .entry(addr)
                    .or_insert(json!({}))
                    .as_object_mut()
                    .unwrap()
                    .insert(nonce.to_string(), json!({}));
            }
            continue;
        }

        // Capture transaction fields
        if let (Some(addr), Some(nonce)) = (&current_addr, &current_nonce) {
            if let Some(entry) = pending.get_mut(addr).and_then(|a| a.get_mut(nonce)) {
                let entry = entry.as_object_mut().unwrap();
                
                if let Some(to_val) = trimmed.strip_prefix("to: Some(") {
                    let to_addr = to_val.trim().trim_matches(',').trim_matches(')');
                    entry.insert("to".to_string(), json!(format!("0x{}", to_addr)));
                } 
                else if trimmed == "to: None," {
                    entry.insert("to".to_string(), Value::Null);
                }
                else if let Some(value) = trimmed.strip_prefix("value: ") {
                    let val = value.trim_matches(',');
                    if let Ok(num) = val.parse::<u64>() {
                        entry.insert("value".to_string(), json!(num));
                    }
                }
                else if let Some(gas) = trimmed.strip_prefix("gas: ") {
                    let gas_val = gas.trim_matches(',');
                    if let Ok(num) = gas_val.parse::<u64>() {
                        entry.insert("gas".to_string(), json!(num));
                    }
                }
                else if let Some(gas_price) = trimmed.strip_prefix("gas_price: ") {
                    let price = gas_price.trim_matches(',');
                    if let Ok(num) = price.parse::<u64>() {
                        entry.insert("gas_price".to_string(), json!(num));
                    }
                }
            }
        }

        // Reset when we hit the end of a block
        if trimmed == "}," || trimmed == "}" {
            if current_nonce.is_some() {
                current_nonce = None;
            } else if current_addr.is_some() {
                current_addr = None;
            }
        }
    }

    Ok(root)
}

fn parse_txpool_content(input: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let mut cleaned = input.to_string();
    
    // Step 1: Remove type wrappers and clean up structure indicators
    let type_wrappers = [
        "TxpoolContent", "AnyRpcTransaction", "WithOtherFields", "Transaction",
        "Recovered", "Ethereum", "Eip1559", "Signed", "TxEip1559", "Call",
        "OnceLock", "PrimitiveSignature", "AccessList", "OtherFields", "AnyRpc",
        "Tx", "Legacy", "TxLegacy", "Eip2930", "TxEip2930", "Eip4844", "TxEip4844",
        "DepositReceipt", "DepositTransaction", "OpDepositReceipt", "SequentialReceipt",
        "Create", "AccessListItem", "TxEip7702", "Eip7702", "Authorization"
    ];
    
    for wrapper in &type_wrappers {
        cleaned = cleaned.replace(&format!("{} {{", wrapper), "{");
        cleaned = cleaned.replace(&format!("{}(", wrapper), "(");
        // Handle cases with newlines
        cleaned = Regex::new(&format!(r"{}\s*\{{", wrapper))?
            .replace_all(&cleaned, "{")
            .to_string();
    }
    
    // Step 2: Handle Some/None and special values
    cleaned = cleaned.replace("Some(", "");
    cleaned = cleaned.replace("None", "null");
    
    // Step 3: Quote field names
    let field_names = [
        "pending", "queued", "inner", "signer", "to", "value", "input", 
        "signature", "y_parity", "r", "s", "hash", "block_hash", "block_number",
        "transaction_index", "effective_gas_price", "other", "chain_id", "nonce",
        "gas_limit", "max_fee_per_gas", "max_priority_fee_per_gas", "tx",
        "access_list", "gas", "gas_price", "from", "data", "type", "v",
        "address", "storage_keys", "blob_versioned_hashes", "max_fee_per_blob_gas",
        "authorization_list"
    ];
    
    for field in &field_names {
        let pattern = format!(r"\b{}\s*:", field);
        let replacement = format!("\"{}\":", field);
        cleaned = Regex::new(&pattern)?
            .replace_all(&cleaned, replacement.as_str())
            .to_string();
    }
    
    // Step 4: Handle Create for contract creation (after field names are quoted)
    cleaned = cleaned.replace("Create,", "null,");
    cleaned = cleaned.replace("Create\n", "null\n");
    
    // Step 5: Handle hex values (including empty 0x)
    cleaned = Regex::new(r"\b0x([0-9a-fA-F]*)\b")?
        .replace_all(&cleaned, "\"0x$1\"")
        .to_string();
    
    // Step 5: Clean up parentheses and fix structure
    // Remove opening parentheses after colons or on lines by themselves
    cleaned = Regex::new(r":\s*\(")?
        .replace_all(&cleaned, ": ")
        .to_string();
    
    // Remove closing parentheses followed by comma
    cleaned = cleaned.replace("),", ",");
    // Remove all parentheses
    cleaned = cleaned.replace(")", "");
    cleaned = cleaned.replace("(", "");
    
    // Step 6: Fix empty objects/arrays
    cleaned = cleaned.replace("\n                                                [],\n                                            ", "[]");
    cleaned = cleaned.replace("[]", "[]");
    cleaned = cleaned.replace(" {}", "{}");
    
    // Remove type names immediately before braces
    cleaned = Regex::new(r"[A-Z][a-zA-Z0-9]*\{")?  
        .replace_all(&cleaned, "{")
        .to_string();
    
    // Also remove standalone type names on their own or followed by whitespace and brace
    cleaned = Regex::new(r#":\s*([A-Z][a-zA-Z0-9]*)\s*\n\s*\{"#)?
        .replace_all(&cleaned, ": {")
        .to_string();
    
    // Step 7: Remove underscores from numbers
    cleaned = Regex::new(r":\s*(\d+)_")?
        .replace_all(&cleaned, ": $1")
        .to_string();
    
    // Step 8: Fix trailing commas (more aggressive)
    // Fix any sequence of closing braces/brackets with trailing commas
    cleaned = Regex::new(r"\},\s*\}")?
        .replace_all(&cleaned, "}}")
        .to_string();
    cleaned = Regex::new(r"\],\s*\}")?
        .replace_all(&cleaned, "]}")
        .to_string();
    cleaned = Regex::new(r"\},\s*\]")?
        .replace_all(&cleaned, "}]")
        .to_string();
    // Standard trailing comma removal
    cleaned = Regex::new(r",\s*\}")?
        .replace_all(&cleaned, "}")
        .to_string();
    cleaned = Regex::new(r",\s*\]")?
        .replace_all(&cleaned, "]")
        .to_string();
    
    // Step 9: Fix any remaining structural issues
    // Remove commas on their own lines
    cleaned = Regex::new(r"\n\s*,\s*\n")?
        .replace_all(&cleaned, "\n")
        .to_string();
    
    // Fix trailing commas after values on their own lines
    cleaned = Regex::new(r#"("0x[0-9a-fA-F]+"|true|false|\d+),\s*\}"#)?
        .replace_all(&cleaned, "$1}")
        .to_string();
    cleaned = Regex::new(r#"("0x[0-9a-fA-F]+"|true|false|\d+),\s*\]"#)?
        .replace_all(&cleaned, "$1]")
        .to_string();
    
    // Final cleanup: process line by line to fix multi-line value issues
    let lines: Vec<&str> = cleaned.lines().collect();
    let mut final_cleaned = String::new();
    
    for i in 0..lines.len() {
        let line = lines[i].trim_end();
        
        // Check if this line ends with a closing brace/bracket followed by comma
        if line.ends_with("},") || line.ends_with("],") {
            // Look ahead to see if the next non-empty line is also a closing brace/bracket
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            
            if j < lines.len() {
                let next_line = lines[j].trim();
                if next_line.starts_with('}') || next_line.starts_with(']') {
                    // Remove the trailing comma
                    final_cleaned.push_str(&line[..line.len()-1]);
                    final_cleaned.push('\n');
                    continue;
                }
            }
        }
        
        final_cleaned.push_str(line);
        final_cleaned.push('\n');
    }
    
    cleaned = final_cleaned;
    
    // Parse as JSON
    match serde_json::from_str(&cleaned) {
        Ok(json) => Ok(json),
        Err(e) => {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs();
            let debug_filename = format!("debug_clean_{}.txt", timestamp);
            std::fs::write(&debug_filename, &cleaned)?;
            
            eprintln!("JSON parse error: {}", e);
            eprintln!("Cleaned output saved to {} for debugging", debug_filename);
            Err(e.into())
        }
    }
}
