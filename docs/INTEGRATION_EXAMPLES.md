# Integration Examples

This guide provides comprehensive integration examples for the Proxima smart contract across multiple programming languages. Each example demonstrates how to build, sign, and broadcast Cosmos SDK transactions.

## Overview

Proxima accepts Cosmos SDK transactions in JSON format, base64-encoded for API submission. The examples below show complete integration workflows including:

- Transaction construction
- Signature creation
- API interaction
- Error handling
- Gas estimation

## JavaScript/TypeScript Integration

### Installation

```bash
npm install @cosmjs/crypto @cosmjs/encoding @cosmjs/math secp256k1
```

### Basic Setup

```typescript
import { fromBase64, toBase64 } from '@cosmjs/encoding';
import { Secp256k1, sha256 } from '@cosmjs/crypto';
import { createHash } from 'crypto';

interface ProximaClient {
  contractId: string;
  rpcUrl: string;
}

class ProximaSDK {
  constructor(private client: ProximaClient) {}

  async broadcastTxSync(txBytes: string) {
    const response = await fetch(`${this.client.rpcUrl}/call`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 'dontcare',
        method: 'call_function',
        params: {
          account_id: this.client.contractId,
          method_name: 'broadcast_tx_sync',
          args_base64: toBase64(JSON.stringify({ tx_bytes: txBytes })),
          finality: 'final'
        }
      })
    });
    
    const result = await response.json();
    return JSON.parse(Buffer.from(result.result.result, 'base64').toString());
  }

  async simulateTx(txBytes: string) {
    const response = await fetch(`${this.client.rpcUrl}/call`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 'dontcare',
        method: 'call_function',
        params: {
          account_id: this.client.contractId,
          method_name: 'simulate_tx',
          args_base64: toBase64(JSON.stringify({ tx_bytes: txBytes })),
          finality: 'final'
        }
      })
    });
    
    const result = await response.json();
    return JSON.parse(Buffer.from(result.result.result, 'base64').toString());
  }
}
```

### Transaction Building

```typescript
interface CosmosTx {
  body: TxBody;
  auth_info: AuthInfo;
  signatures: string[];
}

interface TxBody {
  messages: any[];
  memo: string;
  timeout_height: number;
  extension_options: any[];
  non_critical_extension_options: any[];
}

interface AuthInfo {
  signer_infos: SignerInfo[];
  fee: Fee;
}

interface SignerInfo {
  public_key: any | null;
  mode_info: {
    mode: string;
    multi: any | null;
  };
  sequence: number;
}

interface Fee {
  amount: { denom: string; amount: string }[];
  gas_limit: number;
  payer: string;
  granter: string;
}

class TransactionBuilder {
  static createBankSendTx(
    fromAddress: string,
    toAddress: string,
    amount: string,
    denom: string,
    sequence: number,
    gasLimit: number = 200000,
    feeAmount: string = "1000"
  ): CosmosTx {
    return {
      body: {
        messages: [{
          "@type": "/cosmos.bank.v1beta1.MsgSend",
          from_address: fromAddress,
          to_address: toAddress,
          amount: [{
            denom: denom,
            amount: amount
          }]
        }],
        memo: "",
        timeout_height: 0,
        extension_options: [],
        non_critical_extension_options: []
      },
      auth_info: {
        signer_infos: [{
          public_key: null,
          mode_info: {
            mode: "Direct",
            multi: null
          },
          sequence: sequence
        }],
        fee: {
          amount: [{
            denom: denom,
            amount: feeAmount
          }],
          gas_limit: gasLimit,
          payer: "",
          granter: ""
        }
      },
      signatures: []
    };
  }

  static createStakingDelegateTx(
    delegatorAddress: string,
    validatorAddress: string,
    amount: string,
    denom: string,
    sequence: number
  ): CosmosTx {
    return {
      body: {
        messages: [{
          "@type": "/cosmos.staking.v1beta1.MsgDelegate",
          delegator_address: delegatorAddress,
          validator_address: validatorAddress,
          amount: {
            denom: denom,
            amount: amount
          }
        }],
        memo: "Staking delegation",
        timeout_height: 0,
        extension_options: [],
        non_critical_extension_options: []
      },
      auth_info: {
        signer_infos: [{
          public_key: null,
          mode_info: {
            mode: "Direct",
            multi: null
          },
          sequence: sequence
        }],
        fee: {
          amount: [{
            denom: denom,
            amount: "2000"
          }],
          gas_limit: 300000,
          payer: "",
          granter: ""
        }
      },
      signatures: []
    };
  }
}
```

### Signing and Broadcasting

```typescript
class ProximaTransactionService {
  constructor(private sdk: ProximaSDK, private privateKey: Uint8Array) {}

  async signTransaction(
    tx: CosmosTx,
    chainId: string,
    accountNumber: number
  ): Promise<CosmosTx> {
    // Create sign document
    const signDoc = {
      body_bytes: toBase64(JSON.stringify(tx.body)),
      auth_info_bytes: toBase64(JSON.stringify(tx.auth_info)),
      chain_id: chainId,
      account_number: accountNumber.toString()
    };

    // Create canonical JSON and hash
    const canonicalJson = JSON.stringify(signDoc, Object.keys(signDoc).sort());
    const hash = sha256(Buffer.from(canonicalJson, 'utf8'));

    // Sign with secp256k1
    const signature = await Secp256k1.createSignature(hash, this.privateKey);
    const signatureBytes = new Uint8Array([
      ...signature.r(32),
      ...signature.s(32),
      signature.recovery
    ]);

    // Add signature to transaction
    tx.signatures = [toBase64(signatureBytes)];
    return tx;
  }

  async sendTokens(
    fromAddress: string,
    toAddress: string,
    amount: string,
    denom: string = "unear"
  ) {
    try {
      // Create transaction
      const sequence = await this.getAccountSequence(fromAddress);
      const accountNumber = await this.getAccountNumber(fromAddress);
      
      let tx = TransactionBuilder.createBankSendTx(
        fromAddress,
        toAddress,
        amount,
        denom,
        sequence
      );

      // Simulate for gas estimation
      const encodedTx = toBase64(JSON.stringify(tx));
      const simulation = await this.sdk.simulateTx(encodedTx);
      
      if (simulation.code !== 0) {
        throw new Error(`Simulation failed: ${simulation.raw_log}`);
      }

      // Update gas limit with buffer
      const estimatedGas = parseInt(simulation.gas_used);
      tx.auth_info.fee.gas_limit = Math.ceil(estimatedGas * 1.2);

      // Sign transaction
      tx = await this.signTransaction(tx, "proxima-testnet-1", accountNumber);

      // Broadcast
      const txBytes = toBase64(JSON.stringify(tx));
      const result = await this.sdk.broadcastTxSync(txBytes);

      if (result.code === 0) {
        console.log(`Transaction successful: ${result.txhash}`);
        return result;
      } else {
        throw new Error(`Transaction failed: ${result.raw_log}`);
      }
    } catch (error) {
      console.error('Transaction error:', error);
      throw error;
    }
  }

  async delegateStake(
    delegatorAddress: string,
    validatorAddress: string,
    amount: string,
    denom: string = "unear"
  ) {
    const sequence = await this.getAccountSequence(delegatorAddress);
    const accountNumber = await this.getAccountNumber(delegatorAddress);
    
    let tx = TransactionBuilder.createStakingDelegateTx(
      delegatorAddress,
      validatorAddress,
      amount,
      denom,
      sequence
    );

    // Sign and broadcast
    tx = await this.signTransaction(tx, "proxima-testnet-1", accountNumber);
    const txBytes = toBase64(JSON.stringify(tx));
    
    return await this.sdk.broadcastTxSync(txBytes);
  }

  private async getAccountSequence(address: string): Promise<number> {
    // Mock implementation - replace with actual account query
    return 0;
  }

  private async getAccountNumber(address: string): Promise<number> {
    // Mock implementation - replace with actual account query
    return 1;
  }
}
```

### Complete Example Usage

```typescript
async function main() {
  const client = new ProximaSDK({
    contractId: 'proxima.testnet',
    rpcUrl: 'https://rpc.testnet.near.org'
  });

  // Your secp256k1 private key (32 bytes)
  const privateKey = fromBase64('your-base64-private-key');
  const service = new ProximaTransactionService(client, privateKey);

  try {
    // Send tokens
    const result = await service.sendTokens(
      'cosmos1sender...',
      'cosmos1recipient...',
      '1000000', // 1 NEAR (in microNEAR)
      'unear'
    );
    console.log('Transfer result:', result);

    // Delegate stake
    const delegateResult = await service.delegateStake(
      'cosmos1delegator...',
      'cosmosvaloper1validator...',
      '5000000' // 5 NEAR
    );
    console.log('Delegation result:', delegateResult);
  } catch (error) {
    console.error('Error:', error);
  }
}

main().catch(console.error);
```

## Go Integration

### Dependencies

```go
// go.mod
module proxima-client

go 1.21

require (
    github.com/btcsuite/btcd/btcec/v2 v2.3.2
    github.com/cosmos/cosmos-sdk v0.47.0
    github.com/tendermint/tendermint v0.37.0
)
```

### Client Implementation

```go
package main

import (
    "bytes"
    "crypto/sha256"
    "encoding/base64"
    "encoding/json"
    "fmt"
    "net/http"
    
    "github.com/btcsuite/btcd/btcec/v2"
    "github.com/btcsuite/btcd/btcec/v2/ecdsa"
)

type ProximaClient struct {
    ContractID string
    RPCURL     string
}

type TxResponse struct {
    Height    string `json:"height"`
    TxHash    string `json:"txhash"`
    Code      int    `json:"code"`
    Data      string `json:"data"`
    RawLog    string `json:"raw_log"`
    Info      string `json:"info"`
    GasWanted string `json:"gas_wanted"`
    GasUsed   string `json:"gas_used"`
    Events    []any  `json:"events"`
    Codespace string `json:"codespace"`
}

type CosmosTx struct {
    Body       TxBody     `json:"body"`
    AuthInfo   AuthInfo   `json:"auth_info"`
    Signatures []string   `json:"signatures"`
}

type TxBody struct {
    Messages                    []any  `json:"messages"`
    Memo                        string `json:"memo"`
    TimeoutHeight               int    `json:"timeout_height"`
    ExtensionOptions            []any  `json:"extension_options"`
    NonCriticalExtensionOptions []any  `json:"non_critical_extension_options"`
}

type AuthInfo struct {
    SignerInfos []SignerInfo `json:"signer_infos"`
    Fee         Fee          `json:"fee"`
}

type SignerInfo struct {
    PublicKey any      `json:"public_key"`
    ModeInfo  ModeInfo `json:"mode_info"`
    Sequence  int      `json:"sequence"`
}

type ModeInfo struct {
    Mode  string `json:"mode"`
    Multi any    `json:"multi"`
}

type Fee struct {
    Amount   []Coin `json:"amount"`
    GasLimit int    `json:"gas_limit"`
    Payer    string `json:"payer"`
    Granter  string `json:"granter"`
}

type Coin struct {
    Denom  string `json:"denom"`
    Amount string `json:"amount"`
}

type MsgSend struct {
    Type        string `json:"@type"`
    FromAddress string `json:"from_address"`
    ToAddress   string `json:"to_address"`
    Amount      []Coin `json:"amount"`
}

func (c *ProximaClient) BroadcastTxSync(txBytes string) (*TxResponse, error) {
    payload := map[string]any{
        "jsonrpc": "2.0",
        "id":      "dontcare",
        "method":  "call_function",
        "params": map[string]any{
            "account_id":   c.ContractID,
            "method_name":  "broadcast_tx_sync",
            "args_base64":  base64.StdEncoding.EncodeToString([]byte(fmt.Sprintf(`{"tx_bytes":"%s"}`, txBytes))),
            "finality":     "final",
        },
    }

    jsonData, err := json.Marshal(payload)
    if err != nil {
        return nil, err
    }

    resp, err := http.Post(c.RPCURL, "application/json", bytes.NewBuffer(jsonData))
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()

    var result struct {
        Result struct {
            Result []byte `json:"result"`
        } `json:"result"`
    }

    if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
        return nil, err
    }

    var txResponse TxResponse
    if err := json.Unmarshal(result.Result.Result, &txResponse); err != nil {
        return nil, err
    }

    return &txResponse, nil
}

func CreateBankSendTx(fromAddr, toAddr, amount, denom string, sequence int) *CosmosTx {
    return &CosmosTx{
        Body: TxBody{
            Messages: []any{
                MsgSend{
                    Type:        "/cosmos.bank.v1beta1.MsgSend",
                    FromAddress: fromAddr,
                    ToAddress:   toAddr,
                    Amount: []Coin{{
                        Denom:  denom,
                        Amount: amount,
                    }},
                },
            },
            Memo:                        "",
            TimeoutHeight:               0,
            ExtensionOptions:            []any{},
            NonCriticalExtensionOptions: []any{},
        },
        AuthInfo: AuthInfo{
            SignerInfos: []SignerInfo{{
                PublicKey: nil,
                ModeInfo: ModeInfo{
                    Mode:  "Direct",
                    Multi: nil,
                },
                Sequence: sequence,
            }},
            Fee: Fee{
                Amount: []Coin{{
                    Denom:  denom,
                    Amount: "1000",
                }},
                GasLimit: 200000,
                Payer:    "",
                Granter:  "",
            },
        },
        Signatures: []string{},
    }
}

func SignTransaction(tx *CosmosTx, privateKey *btcec.PrivateKey, chainID string, accountNumber int) error {
    // Create sign document
    bodyBytes, _ := json.Marshal(tx.Body)
    authInfoBytes, _ := json.Marshal(tx.AuthInfo)
    
    signDoc := map[string]any{
        "body_bytes":      base64.StdEncoding.EncodeToString(bodyBytes),
        "auth_info_bytes": base64.StdEncoding.EncodeToString(authInfoBytes),
        "chain_id":        chainID,
        "account_number":  fmt.Sprintf("%d", accountNumber),
    }

    // Create canonical JSON
    canonicalJSON, err := json.Marshal(signDoc)
    if err != nil {
        return err
    }

    // Hash and sign
    hash := sha256.Sum256(canonicalJSON)
    signature := ecdsa.Sign(privateKey, hash[:])

    // Serialize signature (r + s + recovery)
    sigBytes := make([]byte, 65)
    copy(sigBytes[:32], signature.R.Bytes())
    copy(sigBytes[32:64], signature.S.Bytes())
    sigBytes[64] = 0 // recovery ID (simplified)

    tx.Signatures = []string{base64.StdEncoding.EncodeToString(sigBytes)}
    return nil
}

func main() {
    client := &ProximaClient{
        ContractID: "proxima.testnet",
        RPCURL:     "https://rpc.testnet.near.org",
    }

    // Create private key (replace with your key)
    privateKeyBytes := make([]byte, 32) // Your 32-byte private key
    privateKey, _ := btcec.PrivKeyFromBytes(privateKeyBytes)

    // Create transaction
    tx := CreateBankSendTx(
        "cosmos1sender...",
        "cosmos1recipient...",
        "1000000",
        "unear",
        0, // sequence
    )

    // Sign transaction
    if err := SignTransaction(tx, privateKey, "proxima-testnet-1", 1); err != nil {
        panic(err)
    }

    // Encode and broadcast
    txBytes, _ := json.Marshal(tx)
    txBytesB64 := base64.StdEncoding.EncodeToString(txBytes)
    
    result, err := client.BroadcastTxSync(txBytesB64)
    if err != nil {
        panic(err)
    }

    if result.Code == 0 {
        fmt.Println("Transaction successful!")
    } else {
        fmt.Printf("Transaction failed: %s\n", result.RawLog)
    }
}
```

## Python Integration

### Installation

```bash
pip install ecdsa requests base64 hashlib json
```

### Implementation

```python
import base64
import json
import hashlib
import requests
from typing import Dict, List, Any, Optional
from ecdsa import SigningKey, SECP256k1
from ecdsa.util import sigencode_string

class ProximaClient:
    def __init__(self, contract_id: str, rpc_url: str):
        self.contract_id = contract_id
        self.rpc_url = rpc_url

    def broadcast_tx_sync(self, tx_bytes: str) -> Dict[str, Any]:
        payload = {
            "jsonrpc": "2.0",
            "id": "dontcare",
            "method": "call_function",
            "params": {
                "account_id": self.contract_id,
                "method_name": "broadcast_tx_sync",
                "args_base64": base64.b64encode(
                    json.dumps({"tx_bytes": tx_bytes}).encode()
                ).decode(),
                "finality": "final"
            }
        }

        response = requests.post(self.rpc_url, json=payload)
        result = response.json()
        
        return json.loads(base64.b64decode(result["result"]["result"]).decode())

    def simulate_tx(self, tx_bytes: str) -> Dict[str, Any]:
        payload = {
            "jsonrpc": "2.0",
            "id": "dontcare",
            "method": "call_function",
            "params": {
                "account_id": self.contract_id,
                "method_name": "simulate_tx",
                "args_base64": base64.b64encode(
                    json.dumps({"tx_bytes": tx_bytes}).encode()
                ).decode(),
                "finality": "final"
            }
        }

        response = requests.post(self.rpc_url, json=payload)
        result = response.json()
        
        return json.loads(base64.b64decode(result["result"]["result"]).decode())

class TransactionBuilder:
    @staticmethod
    def create_bank_send_tx(
        from_address: str,
        to_address: str,
        amount: str,
        denom: str,
        sequence: int,
        gas_limit: int = 200000,
        fee_amount: str = "1000"
    ) -> Dict[str, Any]:
        return {
            "body": {
                "messages": [{
                    "@type": "/cosmos.bank.v1beta1.MsgSend",
                    "from_address": from_address,
                    "to_address": to_address,
                    "amount": [{
                        "denom": denom,
                        "amount": amount
                    }]
                }],
                "memo": "",
                "timeout_height": 0,
                "extension_options": [],
                "non_critical_extension_options": []
            },
            "auth_info": {
                "signer_infos": [{
                    "public_key": None,
                    "mode_info": {
                        "mode": "Direct",
                        "multi": None
                    },
                    "sequence": sequence
                }],
                "fee": {
                    "amount": [{
                        "denom": denom,
                        "amount": fee_amount
                    }],
                    "gas_limit": gas_limit,
                    "payer": "",
                    "granter": ""
                }
            },
            "signatures": []
        }

    @staticmethod
    def create_staking_delegate_tx(
        delegator_address: str,
        validator_address: str,
        amount: str,
        denom: str,
        sequence: int
    ) -> Dict[str, Any]:
        return {
            "body": {
                "messages": [{
                    "@type": "/cosmos.staking.v1beta1.MsgDelegate",
                    "delegator_address": delegator_address,
                    "validator_address": validator_address,
                    "amount": {
                        "denom": denom,
                        "amount": amount
                    }
                }],
                "memo": "Staking delegation",
                "timeout_height": 0,
                "extension_options": [],
                "non_critical_extension_options": []
            },
            "auth_info": {
                "signer_infos": [{
                    "public_key": None,
                    "mode_info": {
                        "mode": "Direct",
                        "multi": None
                    },
                    "sequence": sequence
                }],
                "fee": {
                    "amount": [{
                        "denom": denom,
                        "amount": "2000"
                    }],
                    "gas_limit": 300000,
                    "payer": "",
                    "granter": ""
                }
            },
            "signatures": []
        }

class ProximaTransactionService:
    def __init__(self, client: ProximaClient, private_key_bytes: bytes):
        self.client = client
        self.private_key = SigningKey.from_string(private_key_bytes, curve=SECP256k1)

    def sign_transaction(
        self,
        tx: Dict[str, Any],
        chain_id: str,
        account_number: int
    ) -> Dict[str, Any]:
        # Create sign document
        sign_doc = {
            "body_bytes": base64.b64encode(json.dumps(tx["body"]).encode()).decode(),
            "auth_info_bytes": base64.b64encode(json.dumps(tx["auth_info"]).encode()).decode(),
            "chain_id": chain_id,
            "account_number": str(account_number)
        }

        # Create canonical JSON and hash
        canonical_json = json.dumps(sign_doc, sort_keys=True, separators=(',', ':'))
        hash_bytes = hashlib.sha256(canonical_json.encode()).digest()

        # Sign with secp256k1
        signature = self.private_key.sign(hash_bytes, sigencode=sigencode_string)
        
        # Add recovery ID (simplified - always 0)
        signature_bytes = signature + b'\x00'

        # Add signature to transaction
        tx["signatures"] = [base64.b64encode(signature_bytes).decode()]
        return tx

    def send_tokens(
        self,
        from_address: str,
        to_address: str,
        amount: str,
        denom: str = "unear"
    ):
        try:
            # Create transaction
            sequence = self.get_account_sequence(from_address)
            account_number = self.get_account_number(from_address)
            
            tx = TransactionBuilder.create_bank_send_tx(
                from_address,
                to_address,
                amount,
                denom,
                sequence
            )

            # Simulate for gas estimation
            encoded_tx = base64.b64encode(json.dumps(tx).encode()).decode()
            simulation = self.client.simulate_tx(encoded_tx)
            
            if simulation["code"] != 0:
                raise Exception(f"Simulation failed: {simulation['raw_log']}")

            # Update gas limit with buffer
            estimated_gas = int(simulation["gas_used"])
            tx["auth_info"]["fee"]["gas_limit"] = int(estimated_gas * 1.2)

            # Sign transaction
            tx = self.sign_transaction(tx, "proxima-testnet-1", account_number)

            # Broadcast
            tx_bytes = base64.b64encode(json.dumps(tx).encode()).decode()
            result = self.client.broadcast_tx_sync(tx_bytes)

            if result["code"] == 0:
                print(f"Transaction successful: {result.get('txhash', 'N/A')}")
                return result
            else:
                raise Exception(f"Transaction failed: {result['raw_log']}")
                
        except Exception as error:
            print(f"Transaction error: {error}")
            raise

    def delegate_stake(
        self,
        delegator_address: str,
        validator_address: str,
        amount: str,
        denom: str = "unear"
    ):
        sequence = self.get_account_sequence(delegator_address)
        account_number = self.get_account_number(delegator_address)
        
        tx = TransactionBuilder.create_staking_delegate_tx(
            delegator_address,
            validator_address,
            amount,
            denom,
            sequence
        )

        # Sign and broadcast
        tx = self.sign_transaction(tx, "proxima-testnet-1", account_number)
        tx_bytes = base64.b64encode(json.dumps(tx).encode()).decode()
        
        return self.client.broadcast_tx_sync(tx_bytes)

    def get_account_sequence(self, address: str) -> int:
        # Mock implementation - replace with actual account query
        return 0

    def get_account_number(self, address: str) -> int:
        # Mock implementation - replace with actual account query
        return 1

# Example usage
def main():
    client = ProximaClient(
        contract_id="proxima.testnet",
        rpc_url="https://rpc.testnet.near.org"
    )

    # Your secp256k1 private key (32 bytes)
    private_key_bytes = base64.b64decode("your-base64-private-key")
    service = ProximaTransactionService(client, private_key_bytes)

    try:
        # Send tokens
        result = service.send_tokens(
            "cosmos1sender...",
            "cosmos1recipient...",
            "1000000",  # 1 NEAR (in microNEAR)
            "unear"
        )
        print("Transfer result:", result)

        # Delegate stake
        delegate_result = service.delegate_stake(
            "cosmos1delegator...",
            "cosmosvaloper1validator...",
            "5000000"  # 5 NEAR
        )
        print("Delegation result:", delegate_result)
        
    except Exception as error:
        print("Error:", error)

if __name__ == "__main__":
    main()
```

## Rust Integration

### Dependencies

```toml
# Cargo.toml
[dependencies]
secp256k1 = { version = "0.27", features = ["rand"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
base64 = "0.21"
sha2 = "0.10"
reqwest = { version = "0.11", features = ["json"] }
tokio = { version = "1.0", features = ["full"] }
```

### Implementation

```rust
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use secp256k1::{Message, Secp256k1, SecretKey};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone)]
pub struct TxResponse {
    pub height: String,
    pub txhash: String,
    pub code: u32,
    pub data: String,
    pub raw_log: String,
    pub info: String,
    pub gas_wanted: String,
    pub gas_used: String,
    pub events: Vec<serde_json::Value>,
    pub codespace: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CosmosTx {
    pub body: TxBody,
    pub auth_info: AuthInfo,
    pub signatures: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TxBody {
    pub messages: Vec<serde_json::Value>,
    pub memo: String,
    pub timeout_height: u64,
    pub extension_options: Vec<serde_json::Value>,
    pub non_critical_extension_options: Vec<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AuthInfo {
    pub signer_infos: Vec<SignerInfo>,
    pub fee: Fee,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SignerInfo {
    pub public_key: Option<serde_json::Value>,
    pub mode_info: ModeInfo,
    pub sequence: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ModeInfo {
    pub mode: String,
    pub multi: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Fee {
    pub amount: Vec<Coin>,
    pub gas_limit: u64,
    pub payer: String,
    pub granter: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Coin {
    pub denom: String,
    pub amount: String,
}

pub struct ProximaClient {
    pub contract_id: String,
    pub rpc_url: String,
    pub client: reqwest::Client,
}

impl ProximaClient {
    pub fn new(contract_id: String, rpc_url: String) -> Self {
        Self {
            contract_id,
            rpc_url,
            client: reqwest::Client::new(),
        }
    }

    pub async fn broadcast_tx_sync(&self, tx_bytes: String) -> Result<TxResponse, Box<dyn std::error::Error>> {
        let args = serde_json::json!({ "tx_bytes": tx_bytes });
        let args_base64 = BASE64.encode(args.to_string());

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "dontcare",
            "method": "call_function",
            "params": {
                "account_id": self.contract_id,
                "method_name": "broadcast_tx_sync",
                "args_base64": args_base64,
                "finality": "final"
            }
        });

        let response = self.client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await?;

        let result: serde_json::Value = response.json().await?;
        let result_bytes = BASE64.decode(result["result"]["result"].as_str().unwrap())?;
        let tx_response: TxResponse = serde_json::from_slice(&result_bytes)?;

        Ok(tx_response)
    }

    pub async fn simulate_tx(&self, tx_bytes: String) -> Result<TxResponse, Box<dyn std::error::Error>> {
        let args = serde_json::json!({ "tx_bytes": tx_bytes });
        let args_base64 = BASE64.encode(args.to_string());

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "dontcare",
            "method": "call_function",
            "params": {
                "account_id": self.contract_id,
                "method_name": "simulate_tx",
                "args_base64": args_base64,
                "finality": "final"
            }
        });

        let response = self.client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await?;

        let result: serde_json::Value = response.json().await?;
        let result_bytes = BASE64.decode(result["result"]["result"].as_str().unwrap())?;
        let tx_response: TxResponse = serde_json::from_slice(&result_bytes)?;

        Ok(tx_response)
    }
}

pub struct TransactionBuilder;

impl TransactionBuilder {
    pub fn create_bank_send_tx(
        from_address: &str,
        to_address: &str,
        amount: &str,
        denom: &str,
        sequence: u64,
    ) -> CosmosTx {
        let message = serde_json::json!({
            "@type": "/cosmos.bank.v1beta1.MsgSend",
            "from_address": from_address,
            "to_address": to_address,
            "amount": [{
                "denom": denom,
                "amount": amount
            }]
        });

        CosmosTx {
            body: TxBody {
                messages: vec![message],
                memo: String::new(),
                timeout_height: 0,
                extension_options: vec![],
                non_critical_extension_options: vec![],
            },
            auth_info: AuthInfo {
                signer_infos: vec![SignerInfo {
                    public_key: None,
                    mode_info: ModeInfo {
                        mode: "Direct".to_string(),
                        multi: None,
                    },
                    sequence,
                }],
                fee: Fee {
                    amount: vec![Coin {
                        denom: denom.to_string(),
                        amount: "1000".to_string(),
                    }],
                    gas_limit: 200000,
                    payer: String::new(),
                    granter: String::new(),
                },
            },
            signatures: vec![],
        }
    }

    pub fn create_staking_delegate_tx(
        delegator_address: &str,
        validator_address: &str,
        amount: &str,
        denom: &str,
        sequence: u64,
    ) -> CosmosTx {
        let message = serde_json::json!({
            "@type": "/cosmos.staking.v1beta1.MsgDelegate",
            "delegator_address": delegator_address,
            "validator_address": validator_address,
            "amount": {
                "denom": denom,
                "amount": amount
            }
        });

        CosmosTx {
            body: TxBody {
                messages: vec![message],
                memo: "Staking delegation".to_string(),
                timeout_height: 0,
                extension_options: vec![],
                non_critical_extension_options: vec![],
            },
            auth_info: AuthInfo {
                signer_infos: vec![SignerInfo {
                    public_key: None,
                    mode_info: ModeInfo {
                        mode: "Direct".to_string(),
                        multi: None,
                    },
                    sequence,
                }],
                fee: Fee {
                    amount: vec![Coin {
                        denom: denom.to_string(),
                        amount: "2000".to_string(),
                    }],
                    gas_limit: 300000,
                    payer: String::new(),
                    granter: String::new(),
                },
            },
            signatures: vec![],
        }
    }
}

pub struct ProximaTransactionService {
    client: ProximaClient,
    private_key: SecretKey,
}

impl ProximaTransactionService {
    pub fn new(client: ProximaClient, private_key_bytes: [u8; 32]) -> Result<Self, Box<dyn std::error::Error>> {
        let private_key = SecretKey::from_slice(&private_key_bytes)?;
        Ok(Self { client, private_key })
    }

    pub fn sign_transaction(
        &self,
        tx: &mut CosmosTx,
        chain_id: &str,
        account_number: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create sign document
        let body_bytes = BASE64.encode(serde_json::to_string(&tx.body)?);
        let auth_info_bytes = BASE64.encode(serde_json::to_string(&tx.auth_info)?);

        let mut sign_doc = HashMap::new();
        sign_doc.insert("body_bytes", body_bytes);
        sign_doc.insert("auth_info_bytes", auth_info_bytes);
        sign_doc.insert("chain_id", chain_id.to_string());
        sign_doc.insert("account_number", account_number.to_string());

        // Create canonical JSON and hash
        let canonical_json = serde_json::to_string(&sign_doc)?;
        let hash = Sha256::digest(canonical_json.as_bytes());

        // Sign with secp256k1
        let secp = Secp256k1::new();
        let message = Message::from_slice(&hash)?;
        let signature = secp.sign_ecdsa(&message, &self.private_key);

        // Serialize signature (64 bytes + recovery ID)
        let mut sig_bytes = Vec::new();
        sig_bytes.extend_from_slice(&signature.serialize_compact());
        sig_bytes.push(0); // recovery ID (simplified)

        tx.signatures = vec![BASE64.encode(sig_bytes)];
        Ok(())
    }

    pub async fn send_tokens(
        &self,
        from_address: &str,
        to_address: &str,
        amount: &str,
        denom: &str,
    ) -> Result<TxResponse, Box<dyn std::error::Error>> {
        // Create transaction
        let sequence = 0; // Mock - replace with actual account query
        let account_number = 1; // Mock - replace with actual account query

        let mut tx = TransactionBuilder::create_bank_send_tx(
            from_address,
            to_address,
            amount,
            denom,
            sequence,
        );

        // Simulate for gas estimation
        let encoded_tx = BASE64.encode(serde_json::to_string(&tx)?);
        let simulation = self.client.simulate_tx(encoded_tx).await?;

        if simulation.code != 0 {
            return Err(format!("Simulation failed: {}", simulation.raw_log).into());
        }

        // Update gas limit with buffer
        let estimated_gas: u64 = simulation.gas_used.parse()?;
        tx.auth_info.fee.gas_limit = (estimated_gas as f64 * 1.2) as u64;

        // Sign transaction
        self.sign_transaction(&mut tx, "proxima-testnet-1", account_number)?;

        // Broadcast
        let tx_bytes = BASE64.encode(serde_json::to_string(&tx)?);
        let result = self.client.broadcast_tx_sync(tx_bytes).await?;

        if result.code == 0 {
            println!("Transaction successful!");
        } else {
            println!("Transaction failed: {}", result.raw_log);
        }

        Ok(result)
    }

    pub async fn delegate_stake(
        &self,
        delegator_address: &str,
        validator_address: &str,
        amount: &str,
        denom: &str,
    ) -> Result<TxResponse, Box<dyn std::error::Error>> {
        let sequence = 0; // Mock - replace with actual account query
        let account_number = 1; // Mock - replace with actual account query

        let mut tx = TransactionBuilder::create_staking_delegate_tx(
            delegator_address,
            validator_address,
            amount,
            denom,
            sequence,
        );

        // Sign and broadcast
        self.sign_transaction(&mut tx, "proxima-testnet-1", account_number)?;
        let tx_bytes = BASE64.encode(serde_json::to_string(&tx)?);
        
        self.client.broadcast_tx_sync(tx_bytes).await
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ProximaClient::new(
        "proxima.testnet".to_string(),
        "https://rpc.testnet.near.org".to_string(),
    );

    // Your secp256k1 private key (32 bytes)
    let private_key_bytes = [0u8; 32]; // Replace with your actual private key
    let service = ProximaTransactionService::new(client, private_key_bytes)?;

    // Send tokens
    let result = service.send_tokens(
        "cosmos1sender...",
        "cosmos1recipient...",
        "1000000", // 1 NEAR (in microNEAR)
        "unear",
    ).await?;
    println!("Transfer result: {:?}", result);

    // Delegate stake
    let delegate_result = service.delegate_stake(
        "cosmos1delegator...",
        "cosmosvaloper1validator...",
        "5000000", // 5 NEAR
        "unear",
    ).await?;
    println!("Delegation result: {:?}", delegate_result);

    Ok(())
}
```

## Error Handling Patterns

### JavaScript/TypeScript Error Handling

```typescript
class ProximaError extends Error {
  constructor(
    public code: number,
    public rawLog: string,
    public codespace: string
  ) {
    super(`Proxima Error ${code}: ${rawLog}`);
  }

  static fromTxResponse(response: TxResponse): ProximaError | null {
    if (response.code === 0) return null;
    return new ProximaError(response.code, response.raw_log, response.codespace);
  }

  isRetryable(): boolean {
    // Sequence errors and gas limit errors are retryable
    return this.code === 3 || this.code === 12;
  }

  getUserFriendlyMessage(): string {
    const messages: Record<number, string> = {
      2: "Transaction format is invalid. Please check your transaction structure.",
      3: "Transaction sequence is outdated. Please refresh and try again.",
      4: "Transaction signature is invalid. Please check your private key.",
      5: "Insufficient funds. Please check your account balance.",
      12: "Transaction requires more gas. Please increase the gas limit.",
    };
    return messages[this.code] || `Transaction failed: ${this.rawLog}`;
  }
}

async function sendTokensWithRetry(service: ProximaTransactionService, ...args: any[]) {
  const maxRetries = 3;
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      return await service.sendTokens(...args);
    } catch (error) {
      if (error instanceof ProximaError && error.isRetryable() && attempt < maxRetries) {
        console.log(`Attempt ${attempt} failed, retrying...`);
        await new Promise(resolve => setTimeout(resolve, 1000 * attempt));
        continue;
      }
      throw error;
    }
  }
}
```

### Go Error Handling

```go
type ProximaError struct {
    Code      int    `json:"code"`
    RawLog    string `json:"raw_log"`
    Codespace string `json:"codespace"`
}

func (e ProximaError) Error() string {
    return fmt.Sprintf("Proxima Error %d: %s", e.Code, e.RawLog)
}

func (e ProximaError) IsRetryable() bool {
    return e.Code == 3 || e.Code == 12 // Sequence or gas errors
}

func SendTokensWithRetry(service *ProximaTransactionService, maxRetries int, args ...interface{}) (*TxResponse, error) {
    for attempt := 1; attempt <= maxRetries; attempt++ {
        result, err := service.SendTokens(args...)
        if err == nil && result.Code == 0 {
            return result, nil
        }
        
        proximaErr := ProximaError{Code: result.Code, RawLog: result.RawLog, Codespace: result.Codespace}
        if proximaErr.IsRetryable() && attempt < maxRetries {
            time.Sleep(time.Duration(attempt) * time.Second)
            continue
        }
        
        return nil, proximaErr
    }
    return nil, fmt.Errorf("max retries exceeded")
}
```

## Best Practices Summary

1. **Always simulate transactions** before broadcasting to estimate gas and validate format
2. **Implement proper error handling** with retry logic for recoverable errors
3. **Validate addresses and amounts** before sending transactions
4. **Use appropriate gas limits** with buffers based on simulation results
5. **Handle sequence numbers correctly** for concurrent transactions
6. **Implement timeout and retry mechanisms** for network resilience
7. **Log transaction details** for debugging and monitoring
8. **Use secure key management** practices in production

This integration guide provides complete examples for building robust Proxima clients across multiple programming languages. Each implementation includes transaction building, signing, broadcasting, and comprehensive error handling.