// src/auth/setup.rs

use super::types::{Administrator, AuthConfig};
use std::io::{self, Write};

pub fn run_initial_setup() -> Result<AuthConfig, Box<dyn std::error::Error>> {
    println!("═══════════════════════════════════════════════════");
    println!("    APOLLO Initial Administrator Setup");
    println!("═══════════════════════════════════════════════════");
    println!();
    println!("You will create TWO administrator accounts.");
    println!("Either admin can log in and reset the other's password.");
    println!();
    
    let mut auth_config = AuthConfig::new();
    
    // Setup Administrator 1
    println!("ADMINISTRATOR 1 SETUP");
    println!("─────────────────────");
    let admin1 = prompt_for_administrator()?;
    auth_config.add_administrator(admin1);
    
    println!();
    
    // Setup Administrator 2
    println!("ADMINISTRATOR 2 SETUP");
    println!("─────────────────────");
    let admin2 = prompt_for_administrator()?;
    auth_config.add_administrator(admin2);
    
    println!();
    println!("✓ Two administrators configured successfully");
    println!("✓ Either administrator can now log in");
    println!();
    
    Ok(auth_config)
}

fn prompt_for_administrator() -> Result<Administrator, Box<dyn std::error::Error>> {
    print!("Full Name: ");
    io::stdout().flush()?;
    let mut full_name = String::new();
    io::stdin().read_line(&mut full_name)?;
    let full_name = full_name.trim().to_string();
    
    if full_name.is_empty() {
        return Err("Full name cannot be empty".into());
    }
    
    print!("Username: ");
    io::stdout().flush()?;
    let mut username = String::new();
    io::stdin().read_line(&mut username)?;
    let username = username.trim().to_string();
    
    if username.is_empty() {
        return Err("Username cannot be empty".into());
    }
    
    // Password with confirmation
    let password = rpassword::prompt_password("Password: ")?;
    let confirm = rpassword::prompt_password("Confirm Password: ")?;
    
    if password != confirm {
        return Err("Passwords do not match".into());
    }
    
    if password.len() < 8 {
        return Err("Password must be at least 8 characters".into());
    }
    
    // Hash the password
    let password_hash = bcrypt::hash(&password, bcrypt::DEFAULT_COST)?;
    
    println!("✓ Administrator '{}' created", username);
    
    Ok(Administrator {
        username,
        password_hash,
        full_name,
        created_at: chrono::Utc::now().to_rfc3339(),
        last_login: None,
    })
}