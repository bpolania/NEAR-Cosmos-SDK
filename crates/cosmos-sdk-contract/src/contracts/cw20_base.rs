/// Complete CW20 Token Implementation using CosmWasm Compatibility Layer
/// 
/// This is a full-featured CW20 token contract that uses standard cosmwasm_std imports
/// and demonstrates how existing CosmWasm contracts can run on NEAR with our compatibility layer.

use crate::modules::cosmwasm::types as cosmwasm_std;
use cosmwasm_std::{
    to_binary, from_slice, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, StdError,
    Uint128, Addr, Storage,
};
use serde::{Deserialize, Serialize};
// use std::collections::HashMap; // Not needed currently

// ============================================================================
// Contract State
// ============================================================================

/// The main CW20 contract state
pub struct Cw20Contract;

// Storage keys
const TOKEN_INFO_KEY: &[u8] = b"token_info";
const MINTER_KEY: &[u8] = b"minter";
const MARKETING_KEY: &[u8] = b"marketing";
const _LOGO_KEY: &[u8] = b"logo";

// ============================================================================
// Messages
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub initial_balances: Vec<Cw20Coin>,
    pub mint: Option<MinterResponse>,
    pub marketing: Option<InstantiateMarketingInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Cw20Coin {
    pub address: String,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MinterResponse {
    pub minter: String,
    pub cap: Option<Uint128>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMarketingInfo {
    pub project: Option<String>,
    pub description: Option<String>,
    pub marketing: Option<String>,
    pub logo: Option<Logo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Logo {
    Url(String),
    Embedded(EmbeddedLogo),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddedLogo {
    Svg(Binary),
    Png(Binary),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Transfer tokens to another account
    Transfer { recipient: String, amount: Uint128 },
    /// Burn tokens from sender's account
    Burn { amount: Uint128 },
    /// Send tokens to a contract with a message
    Send { contract: String, amount: Uint128, msg: Binary },
    /// Mint tokens (only minter can call this)
    Mint { recipient: String, amount: Uint128 },
    /// Increase allowance for spender
    IncreaseAllowance { spender: String, amount: Uint128, expires: Option<Expiration> },
    /// Decrease allowance for spender
    DecreaseAllowance { spender: String, amount: Uint128, expires: Option<Expiration> },
    /// Transfer tokens from owner to recipient (requires allowance)
    TransferFrom { owner: String, recipient: String, amount: Uint128 },
    /// Send tokens from owner to contract (requires allowance)
    SendFrom { owner: String, contract: String, amount: Uint128, msg: Binary },
    /// Burn tokens from owner's account (requires allowance)
    BurnFrom { owner: String, amount: Uint128 },
    /// Update minter (only current minter can call this)
    UpdateMinter { new_minter: Option<String> },
    /// Update marketing info (only marketing account can call this)
    UpdateMarketing { project: Option<String>, description: Option<String>, marketing: Option<String> },
    /// Upload logo (only marketing account can call this)
    UploadLogo(Logo),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Expiration {
    AtHeight(u64),
    AtTime(u64),
    Never {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get token balance for address
    Balance { address: String },
    /// Get token information
    TokenInfo {},
    /// Get minter information
    Minter {},
    /// Get allowance information
    Allowance { owner: String, spender: String },
    /// Get all allowances for an owner
    AllAllowances { owner: String, start_after: Option<String>, limit: Option<u32> },
    /// Get all accounts with balances
    AllAccounts { start_after: Option<String>, limit: Option<u32> },
    /// Get marketing information
    MarketingInfo {},
    /// Download logo
    DownloadLogo {},
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BalanceResponse {
    pub balance: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TokenInfoResponse {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AllowanceResponse {
    pub allowance: Uint128,
    pub expires: Expiration,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AllAllowancesResponse {
    pub allowances: Vec<AllowanceInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AllowanceInfo {
    pub spender: String,
    pub allowance: Uint128,
    pub expires: Expiration,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AllAccountsResponse {
    pub accounts: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MarketingInfoResponse {
    pub project: Option<String>,
    pub description: Option<String>,
    pub marketing: Option<String>,
    pub logo: Option<LogoInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LogoInfo {
    Url(String),
    Embedded,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DownloadLogoResponse {
    pub mime_type: String,
    pub data: Binary,
}

// ============================================================================
// State Types
// ============================================================================

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MinterData {
    pub minter: Addr,
    pub cap: Option<Uint128>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MarketingInfo {
    pub project: Option<String>,
    pub description: Option<String>,
    pub marketing: Option<Addr>,
    pub logo: Option<Logo>,
}

// ============================================================================
// Contract Implementation
// ============================================================================

impl Cw20Contract {
    pub fn instantiate(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: InstantiateMsg,
    ) -> StdResult<Response> {
        // Validate and create token info
        let mut total_supply = Uint128::zero();
        
        // Create token info
        let token_info = TokenInfo {
            name: msg.name.clone(),
            symbol: msg.symbol.clone(),
            decimals: msg.decimals,
            total_supply,
        };
        
        // Store token info
        deps.storage.set(TOKEN_INFO_KEY, &to_binary(&token_info)?.as_slice());
        
        // Set initial balances
        for balance in msg.initial_balances {
            let addr = deps.api.addr_validate(&balance.address)?;
            if balance.amount > Uint128::zero() {
                set_balance(deps.storage, &addr, balance.amount)?;
                total_supply += balance.amount;
            }
        }
        
        // Update total supply
        let token_info = TokenInfo {
            name: msg.name.clone(),
            symbol: msg.symbol.clone(),
            decimals: msg.decimals,
            total_supply,
        };
        deps.storage.set(TOKEN_INFO_KEY, &to_binary(&token_info)?.as_slice());
        
        // Set minter if provided
        if let Some(mint) = msg.mint {
            let minter_addr = deps.api.addr_validate(&mint.minter)?;
            let minter_data = MinterData {
                minter: minter_addr,
                cap: mint.cap,
            };
            deps.storage.set(MINTER_KEY, &to_binary(&minter_data)?.as_slice());
        }
        
        // Set marketing info if provided
        if let Some(marketing) = msg.marketing {
            let marketing_addr = marketing.marketing.as_ref()
                .map(|addr| deps.api.addr_validate(addr))
                .transpose()?;
            
            let marketing_info = MarketingInfo {
                project: marketing.project,
                description: marketing.description,
                marketing: marketing_addr,
                logo: marketing.logo,
            };
            deps.storage.set(MARKETING_KEY, &to_binary(&marketing_info)?.as_slice());
        }
        
        Ok(Response::new()
            .add_attribute("action", "instantiate")
            .add_attribute("name", msg.name)
            .add_attribute("symbol", msg.symbol)
            .add_attribute("decimals", msg.decimals.to_string())
            .add_attribute("total_supply", total_supply.to_string()))
    }
    
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> StdResult<Response> {
        match msg {
            ExecuteMsg::Transfer { recipient, amount } => {
                execute_transfer(deps, env, info, recipient, amount)
            }
            ExecuteMsg::Burn { amount } => {
                execute_burn(deps, env, info, amount)
            }
            ExecuteMsg::Send { contract, amount, msg } => {
                execute_send(deps, env, info, contract, amount, msg)
            }
            ExecuteMsg::Mint { recipient, amount } => {
                execute_mint(deps, env, info, recipient, amount)
            }
            ExecuteMsg::IncreaseAllowance { spender, amount, expires } => {
                execute_increase_allowance(deps, env, info, spender, amount, expires)
            }
            ExecuteMsg::DecreaseAllowance { spender, amount, expires } => {
                execute_decrease_allowance(deps, env, info, spender, amount, expires)
            }
            ExecuteMsg::TransferFrom { owner, recipient, amount } => {
                execute_transfer_from(deps, env, info, owner, recipient, amount)
            }
            ExecuteMsg::SendFrom { owner, contract, amount, msg } => {
                execute_send_from(deps, env, info, owner, contract, amount, msg)
            }
            ExecuteMsg::BurnFrom { owner, amount } => {
                execute_burn_from(deps, env, info, owner, amount)
            }
            ExecuteMsg::UpdateMinter { new_minter } => {
                execute_update_minter(deps, env, info, new_minter)
            }
            ExecuteMsg::UpdateMarketing { project, description, marketing } => {
                execute_update_marketing(deps, env, info, project, description, marketing)
            }
            ExecuteMsg::UploadLogo(logo) => {
                execute_upload_logo(deps, env, info, logo)
            }
        }
    }
    
    pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
        match msg {
            QueryMsg::Balance { address } => {
                let addr = deps.api.addr_validate(&address)?;
                let balance = get_balance(deps.storage, &addr)?;
                to_binary(&BalanceResponse { balance })
            }
            QueryMsg::TokenInfo {} => {
                let info = get_token_info(deps.storage)?;
                to_binary(&TokenInfoResponse {
                    name: info.name,
                    symbol: info.symbol,
                    decimals: info.decimals,
                    total_supply: info.total_supply,
                })
            }
            QueryMsg::Minter {} => {
                let minter = get_minter(deps.storage)?;
                to_binary(&MinterResponse {
                    minter: minter.minter.to_string(),
                    cap: minter.cap,
                })
            }
            QueryMsg::Allowance { owner, spender } => {
                let owner_addr = deps.api.addr_validate(&owner)?;
                let spender_addr = deps.api.addr_validate(&spender)?;
                let allowance = get_allowance(deps.storage, &owner_addr, &spender_addr)?;
                to_binary(&AllowanceResponse {
                    allowance: allowance.allowance,
                    expires: allowance.expires,
                })
            }
            QueryMsg::AllAllowances { owner, start_after, limit } => {
                query_all_allowances(deps, owner, start_after, limit)
            }
            QueryMsg::AllAccounts { start_after, limit } => {
                query_all_accounts(deps, start_after, limit)
            }
            QueryMsg::MarketingInfo {} => {
                query_marketing_info(deps)
            }
            QueryMsg::DownloadLogo {} => {
                query_download_logo(deps)
            }
        }
    }
}

// ============================================================================
// Execute Handlers
// ============================================================================

fn execute_transfer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> StdResult<Response> {
    if amount == Uint128::zero() {
        return Err(StdError::generic_err("Invalid zero amount"));
    }
    
    let rcpt_addr = deps.api.addr_validate(&recipient)?;
    
    // Decrease sender balance
    let sender_balance = get_balance(deps.storage, &info.sender)?;
    if sender_balance < amount {
        return Err(StdError::generic_err("Insufficient funds"));
    }
    set_balance(deps.storage, &info.sender, sender_balance - amount)?;
    
    // Increase recipient balance
    let rcpt_balance = get_balance(deps.storage, &rcpt_addr)?;
    set_balance(deps.storage, &rcpt_addr, rcpt_balance + amount)?;
    
    Ok(Response::new()
        .add_attribute("action", "transfer")
        .add_attribute("from", info.sender.as_str())
        .add_attribute("to", recipient)
        .add_attribute("amount", amount.to_string()))
}

fn execute_burn(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> StdResult<Response> {
    if amount == Uint128::zero() {
        return Err(StdError::generic_err("Invalid zero amount"));
    }
    
    // Decrease sender balance
    let sender_balance = get_balance(deps.storage, &info.sender)?;
    if sender_balance < amount {
        return Err(StdError::generic_err("Insufficient funds"));
    }
    set_balance(deps.storage, &info.sender, sender_balance - amount)?;
    
    // Decrease total supply
    let mut token_info = get_token_info(deps.storage)?;
    token_info.total_supply = token_info.total_supply.checked_sub(amount)?;
    deps.storage.set(TOKEN_INFO_KEY, &to_binary(&token_info)?.as_slice());
    
    Ok(Response::new()
        .add_attribute("action", "burn")
        .add_attribute("from", info.sender.as_str())
        .add_attribute("amount", amount.to_string()))
}

fn execute_send(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    contract: String,
    amount: Uint128,
    msg: Binary,
) -> StdResult<Response> {
    if amount == Uint128::zero() {
        return Err(StdError::generic_err("Invalid zero amount"));
    }
    
    let rcpt_addr = deps.api.addr_validate(&contract)?;
    
    // Decrease sender balance
    let sender_balance = get_balance(deps.storage, &info.sender)?;
    if sender_balance < amount {
        return Err(StdError::generic_err("Insufficient funds"));
    }
    set_balance(deps.storage, &info.sender, sender_balance - amount)?;
    
    // Increase recipient balance
    let rcpt_balance = get_balance(deps.storage, &rcpt_addr)?;
    set_balance(deps.storage, &rcpt_addr, rcpt_balance + amount)?;
    
    // Note: In a real implementation, we would create a submessage to call the contract
    // For our compatibility layer demo, we'll just log the message
    
    Ok(Response::new()
        .add_attribute("action", "send")
        .add_attribute("from", info.sender.as_str())
        .add_attribute("to", contract)
        .add_attribute("amount", amount.to_string())
        .add_attribute("msg", msg.to_base64()))
}

fn execute_mint(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> StdResult<Response> {
    if amount == Uint128::zero() {
        return Err(StdError::generic_err("Invalid zero amount"));
    }
    
    // Check if sender is authorized minter
    let minter = get_minter(deps.storage)?;
    if info.sender != minter.minter {
        return Err(StdError::generic_err("Unauthorized"));
    }
    
    // Check mint cap
    if let Some(cap) = minter.cap {
        let token_info = get_token_info(deps.storage)?;
        if token_info.total_supply + amount > cap {
            return Err(StdError::generic_err("Cannot exceed minting cap"));
        }
    }
    
    let rcpt_addr = deps.api.addr_validate(&recipient)?;
    
    // Increase recipient balance
    let rcpt_balance = get_balance(deps.storage, &rcpt_addr)?;
    set_balance(deps.storage, &rcpt_addr, rcpt_balance + amount)?;
    
    // Increase total supply
    let mut token_info = get_token_info(deps.storage)?;
    token_info.total_supply += amount;
    deps.storage.set(TOKEN_INFO_KEY, &to_binary(&token_info)?.as_slice());
    
    Ok(Response::new()
        .add_attribute("action", "mint")
        .add_attribute("to", recipient)
        .add_attribute("amount", amount.to_string()))
}

fn execute_increase_allowance(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    spender: String,
    amount: Uint128,
    expires: Option<Expiration>,
) -> StdResult<Response> {
    let spender_addr = deps.api.addr_validate(&spender)?;
    let expires = expires.unwrap_or(Expiration::Never {});
    
    let mut allowance = get_allowance(deps.storage, &info.sender, &spender_addr)?;
    allowance.allowance += amount;
    allowance.expires = expires;
    
    set_allowance(deps.storage, &info.sender, &spender_addr, &allowance)?;
    
    Ok(Response::new()
        .add_attribute("action", "increase_allowance")
        .add_attribute("owner", info.sender.as_str())
        .add_attribute("spender", spender)
        .add_attribute("amount", amount.to_string()))
}

fn execute_decrease_allowance(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    spender: String,
    amount: Uint128,
    expires: Option<Expiration>,
) -> StdResult<Response> {
    let spender_addr = deps.api.addr_validate(&spender)?;
    let expires = expires.unwrap_or(Expiration::Never {});
    
    let mut allowance = get_allowance(deps.storage, &info.sender, &spender_addr)?;
    allowance.allowance = allowance.allowance.checked_sub(amount)?;
    allowance.expires = expires;
    
    set_allowance(deps.storage, &info.sender, &spender_addr, &allowance)?;
    
    Ok(Response::new()
        .add_attribute("action", "decrease_allowance")
        .add_attribute("owner", info.sender.as_str())
        .add_attribute("spender", spender)
        .add_attribute("amount", amount.to_string()))
}

fn execute_transfer_from(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: String,
    recipient: String,
    amount: Uint128,
) -> StdResult<Response> {
    if amount == Uint128::zero() {
        return Err(StdError::generic_err("Invalid zero amount"));
    }
    
    let owner_addr = deps.api.addr_validate(&owner)?;
    let rcpt_addr = deps.api.addr_validate(&recipient)?;
    
    // Check and decrease allowance
    let mut allowance = get_allowance(deps.storage, &owner_addr, &info.sender)?;
    if allowance.allowance < amount {
        return Err(StdError::generic_err("Insufficient allowance"));
    }
    allowance.allowance -= amount;
    set_allowance(deps.storage, &owner_addr, &info.sender, &allowance)?;
    
    // Check and decrease owner balance
    let owner_balance = get_balance(deps.storage, &owner_addr)?;
    if owner_balance < amount {
        return Err(StdError::generic_err("Insufficient funds"));
    }
    set_balance(deps.storage, &owner_addr, owner_balance - amount)?;
    
    // Increase recipient balance
    let rcpt_balance = get_balance(deps.storage, &rcpt_addr)?;
    set_balance(deps.storage, &rcpt_addr, rcpt_balance + amount)?;
    
    Ok(Response::new()
        .add_attribute("action", "transfer_from")
        .add_attribute("from", owner)
        .add_attribute("to", recipient)
        .add_attribute("by", info.sender.as_str())
        .add_attribute("amount", amount.to_string()))
}

fn execute_send_from(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: String,
    contract: String,
    amount: Uint128,
    msg: Binary,
) -> StdResult<Response> {
    if amount == Uint128::zero() {
        return Err(StdError::generic_err("Invalid zero amount"));
    }
    
    let owner_addr = deps.api.addr_validate(&owner)?;
    let rcpt_addr = deps.api.addr_validate(&contract)?;
    
    // Check and decrease allowance
    let mut allowance = get_allowance(deps.storage, &owner_addr, &info.sender)?;
    if allowance.allowance < amount {
        return Err(StdError::generic_err("Insufficient allowance"));
    }
    allowance.allowance -= amount;
    set_allowance(deps.storage, &owner_addr, &info.sender, &allowance)?;
    
    // Check and decrease owner balance
    let owner_balance = get_balance(deps.storage, &owner_addr)?;
    if owner_balance < amount {
        return Err(StdError::generic_err("Insufficient funds"));
    }
    set_balance(deps.storage, &owner_addr, owner_balance - amount)?;
    
    // Increase recipient balance
    let rcpt_balance = get_balance(deps.storage, &rcpt_addr)?;
    set_balance(deps.storage, &rcpt_addr, rcpt_balance + amount)?;
    
    Ok(Response::new()
        .add_attribute("action", "send_from")
        .add_attribute("from", owner)
        .add_attribute("to", contract)
        .add_attribute("by", info.sender.as_str())
        .add_attribute("amount", amount.to_string())
        .add_attribute("msg", msg.to_base64()))
}

fn execute_burn_from(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: String,
    amount: Uint128,
) -> StdResult<Response> {
    if amount == Uint128::zero() {
        return Err(StdError::generic_err("Invalid zero amount"));
    }
    
    let owner_addr = deps.api.addr_validate(&owner)?;
    
    // Check and decrease allowance
    let mut allowance = get_allowance(deps.storage, &owner_addr, &info.sender)?;
    if allowance.allowance < amount {
        return Err(StdError::generic_err("Insufficient allowance"));
    }
    allowance.allowance -= amount;
    set_allowance(deps.storage, &owner_addr, &info.sender, &allowance)?;
    
    // Check and decrease owner balance
    let owner_balance = get_balance(deps.storage, &owner_addr)?;
    if owner_balance < amount {
        return Err(StdError::generic_err("Insufficient funds"));
    }
    set_balance(deps.storage, &owner_addr, owner_balance - amount)?;
    
    // Decrease total supply
    let mut token_info = get_token_info(deps.storage)?;
    token_info.total_supply = token_info.total_supply.checked_sub(amount)?;
    deps.storage.set(TOKEN_INFO_KEY, &to_binary(&token_info)?.as_slice());
    
    Ok(Response::new()
        .add_attribute("action", "burn_from")
        .add_attribute("from", owner)
        .add_attribute("by", info.sender.as_str())
        .add_attribute("amount", amount.to_string()))
}

fn execute_update_minter(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_minter: Option<String>,
) -> StdResult<Response> {
    let minter = get_minter(deps.storage)?;
    if info.sender != minter.minter {
        return Err(StdError::generic_err("Unauthorized"));
    }
    
    let minter_str = new_minter.as_ref().map(|s| s.as_str()).unwrap_or("none").to_string();
    
    match new_minter {
        Some(addr) => {
            let new_minter_addr = deps.api.addr_validate(&addr)?;
            let new_minter_data = MinterData {
                minter: new_minter_addr,
                cap: minter.cap,
            };
            deps.storage.set(MINTER_KEY, &to_binary(&new_minter_data)?.as_slice());
        }
        None => {
            deps.storage.remove(MINTER_KEY);
        }
    }
    
    Ok(Response::new()
        .add_attribute("action", "update_minter")
        .add_attribute("new_minter", minter_str))
}

fn execute_update_marketing(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    project: Option<String>,
    description: Option<String>,
    marketing: Option<String>,
) -> StdResult<Response> {
    let mut marketing_info = get_marketing_info(deps.storage)?;
    
    // Check authorization
    if let Some(ref marketing_addr) = marketing_info.marketing {
        if &info.sender != marketing_addr {
            return Err(StdError::generic_err("Unauthorized"));
        }
    } else {
        return Err(StdError::generic_err("No marketing info set"));
    }
    
    // Update fields
    if let Some(project) = project {
        marketing_info.project = Some(project);
    }
    if let Some(description) = description {
        marketing_info.description = Some(description);
    }
    if let Some(marketing) = marketing {
        let marketing_addr = deps.api.addr_validate(&marketing)?;
        marketing_info.marketing = Some(marketing_addr);
    }
    
    deps.storage.set(MARKETING_KEY, &to_binary(&marketing_info)?.as_slice());
    
    Ok(Response::new().add_attribute("action", "update_marketing"))
}

fn execute_upload_logo(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    logo: Logo,
) -> StdResult<Response> {
    let mut marketing_info = get_marketing_info(deps.storage)?;
    
    // Check authorization
    if let Some(ref marketing_addr) = marketing_info.marketing {
        if &info.sender != marketing_addr {
            return Err(StdError::generic_err("Unauthorized"));
        }
    } else {
        return Err(StdError::generic_err("No marketing info set"));
    }
    
    marketing_info.logo = Some(logo);
    deps.storage.set(MARKETING_KEY, &to_binary(&marketing_info)?.as_slice());
    
    Ok(Response::new().add_attribute("action", "upload_logo"))
}

// ============================================================================
// Query Handlers
// ============================================================================

fn query_all_allowances(
    deps: Deps,
    owner: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let _owner_addr = deps.api.addr_validate(&owner)?;
    let _limit = limit.unwrap_or(30).min(100) as usize;
    
    let _start = start_after.as_ref().map(String::as_str);
    
    // In a real implementation, we would iterate over stored allowances
    // For our compatibility layer demo, we'll return an empty list
    let allowances = vec![];
    
    to_binary(&AllAllowancesResponse { allowances })
}

fn query_all_accounts(
    _deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let _limit = limit.unwrap_or(30).min(100) as usize;
    
    let _start = start_after.as_ref().map(String::as_str);
    
    // In a real implementation, we would iterate over stored balances
    // For our compatibility layer demo, we'll return an empty list
    let accounts = vec![];
    
    to_binary(&AllAccountsResponse { accounts })
}

fn query_marketing_info(deps: Deps) -> StdResult<Binary> {
    let marketing_info = get_marketing_info(deps.storage)?;
    
    let logo = marketing_info.logo.map(|logo| match logo {
        Logo::Url(url) => LogoInfo::Url(url),
        Logo::Embedded(_) => LogoInfo::Embedded,
    });
    
    to_binary(&MarketingInfoResponse {
        project: marketing_info.project,
        description: marketing_info.description,
        marketing: marketing_info.marketing.map(|addr| addr.to_string()),
        logo,
    })
}

fn query_download_logo(deps: Deps) -> StdResult<Binary> {
    let marketing_info = get_marketing_info(deps.storage)?;
    
    match marketing_info.logo {
        Some(Logo::Embedded(EmbeddedLogo::Svg(data))) => {
            to_binary(&DownloadLogoResponse {
                mime_type: "image/svg+xml".to_string(),
                data,
            })
        }
        Some(Logo::Embedded(EmbeddedLogo::Png(data))) => {
            to_binary(&DownloadLogoResponse {
                mime_type: "image/png".to_string(),
                data,
            })
        }
        _ => Err(StdError::not_found("Logo")),
    }
}

// ============================================================================
// Storage Helpers
// ============================================================================

fn get_balance(storage: &dyn Storage, addr: &Addr) -> StdResult<Uint128> {
    let key = balance_key(addr);
    match storage.get(&key) {
        Some(data) => from_slice(&data),
        None => Ok(Uint128::zero()),
    }
}

fn set_balance(storage: &mut dyn Storage, addr: &Addr, amount: Uint128) -> StdResult<()> {
    let key = balance_key(addr);
    if amount.is_zero() {
        storage.remove(&key);
    } else {
        storage.set(&key, &to_binary(&amount)?.as_slice());
    }
    Ok(())
}

fn get_allowance(storage: &dyn Storage, owner: &Addr, spender: &Addr) -> StdResult<AllowanceInfo> {
    let key = allowance_key(owner, spender);
    match storage.get(&key) {
        Some(data) => from_slice(&data),
        None => Ok(AllowanceInfo {
            spender: spender.to_string(),
            allowance: Uint128::zero(),
            expires: Expiration::Never {},
        }),
    }
}

fn set_allowance(
    storage: &mut dyn Storage,
    owner: &Addr,
    spender: &Addr,
    allowance: &AllowanceInfo,
) -> StdResult<()> {
    let key = allowance_key(owner, spender);
    if allowance.allowance.is_zero() {
        storage.remove(&key);
    } else {
        storage.set(&key, &to_binary(allowance)?.as_slice());
    }
    Ok(())
}

fn get_token_info(storage: &dyn Storage) -> StdResult<TokenInfo> {
    match storage.get(TOKEN_INFO_KEY) {
        Some(data) => from_slice(&data),
        None => Err(StdError::not_found("token_info")),
    }
}

fn get_minter(storage: &dyn Storage) -> StdResult<MinterData> {
    match storage.get(MINTER_KEY) {
        Some(data) => from_slice(&data),
        None => Err(StdError::not_found("minter")),
    }
}

fn get_marketing_info(storage: &dyn Storage) -> StdResult<MarketingInfo> {
    match storage.get(MARKETING_KEY) {
        Some(data) => from_slice(&data),
        None => Err(StdError::not_found("marketing_info")),
    }
}

// Storage key generators
fn balance_key(addr: &Addr) -> Vec<u8> {
    format!("balance:{}", addr.as_str()).into_bytes()
}

fn allowance_key(owner: &Addr, spender: &Addr) -> Vec<u8> {
    format!("allowance:{}:{}", owner.as_str(), spender.as_str()).into_bytes()
}