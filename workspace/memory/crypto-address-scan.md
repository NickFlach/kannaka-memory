# Crypto Address Scan Report
**Date:** 2026-02-12
**Scope:** All ~87 repos in C:\Users\nickf\Source (excluding node_modules, .git, vendor, dist, build, .next)

## Summary
Scan complete. Found hundreds of matches across repos. Most are deployed contract addresses, well-known protocol addresses, zero addresses, test fixtures, or compiled bytecode. **7 potentially personal/sensitive wallet addresses** identified that may need scrubbing.

## 🚨 Potentially Sensitive Personal Addresses

### 1. `0x618d855C2F32f1C9343624111b8bEd20eEccdf53`
- **Repos:** ChessAI
- **Files:** `.github/FUNDING.yml`, `client/src/components/support-banner.tsx`, `client/src/pages/landing.tsx`, `client/src/pages/pricing.tsx`
- **Context:** ETH/ERC-20/Polygon/Base/Arbitrum donation address
- **Risk:** HIGH — public donation address linked to identity

### 2. `0x3e4dFF8955C0Da6fa9709E1bdDb092D580Fb2304`
- **Repos:** flaukowski (profile README)
- **Files:** `README.md`
- **Context:** "donations go here for eth"
- **Risk:** HIGH — personal donation address on profile repo

### 3. `0x5C2FFf175128ADdD8B982ecF87Fb4f61150095aA`
- **Repos:** flaukowski (README), FlaukowskiAgent (Configuration.tsx, Login.tsx, Portfolio.tsx, routes.ts)
- **Files:** 6+ files
- **Context:** NEO X target/payment address
- **Risk:** HIGH — personal address hardcoded as payment target

### 4. `0x7C29b9Bc9f7CA06DB45E5558c6DEe84f4dd01efb`
- **Repos:** pitchfork-echo-studio
- **Files:** `FUNDING.md`, `src/components/DeveloperFunding.tsx`, `src/lib/api.ts`
- **Context:** Developer wallet for donations
- **Risk:** HIGH — personal developer wallet

### 5. `0xB41C12E86EE878c918c38Ae5a7E08AA5509B2085`
- **Repos:** NinjaPortal, PSRS
- **Files:** Pasted API response text files (attached_assets)
- **Context:** Wallet address in pasted HTTP response headers
- **Risk:** MEDIUM — appears in test data/pasted content

### 6. `0x742d35Cc6634C0532925a3b844Bc9e7595f3bF8d`
- **Repos:** FlaukowskiFashion
- **Files:** `server/routes.ts`
- **Context:** RECEIVER_ADDRESS constant
- **Risk:** MEDIUM — hardcoded receiver (may be placeholder)

### 7. `0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb7`
- **Repos:** PFORK_MCP
- **Files:** `mcp-server/README.md`
- **Context:** ADMIN_ADDRESSES / DEVELOPER_ADDRESS in example config
- **Risk:** LOW-MEDIUM — may be example/placeholder (similar to known Bitfinex address pattern)

## ✅ Safe Categories (No Action Needed)

### Deployed Smart Contract Addresses
These are public contract addresses meant to be in code:
- PFORK Token: `0x216490C8E6b33b4d8A2390dADcf9f433E30da60F` (20+ repos)
- AMOR contracts: 5 addresses across AMOR repo
- Ferry contracts: `0xDE7cF1Dd14b613db5A4727A59ad1Cc1ba6f47a86`, `0x81aC8AEDdaC85aA14011ab88944aA147472aC525`
- DEX contract: `0x62Cf7e56D5AA77002995F4f8037bCa649988cE62`
- Hardhat local deployment addresses (pitchfork-echo-studio)
- Treasury addresses, Faucet addresses, NFT contracts
- SpaceChildCollective blockchain config

### Well-Known Protocol Addresses
- WETH: `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2`
- Uniswap V3 Router/Factory addresses
- USDC: `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48`
- Neo Name Service hashes
- secp256k1 curve parameters (ChessAI zkp-crypto)

### Zero/Dead/Test Addresses
- `0x0000000000000000000000000000000000000000` (many repos)
- `0x000000000000000000000000000000000000dEaD`
- `0x1234567890123456789012345678901234567890` (placeholder)
- `0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266` (Hardhat default deployer)

### Compiled Bytecode & Artifacts
- pitchfork-echo-studio `artifacts/` directory — compiled Solidity contracts
- pitchfork-echo-studio `build-info/` — Hardhat build artifacts
- collaborateESCO webpack bundles

### Upstream Library/Fork Data
- neo repo — benchmark/test JSON data (Neo blockchain RPC test cases)
- v3-core — Uniswap V3 test config
- spoon-spacechild-core — trade aggregator with well-known DEX addresses

### Factory Contract Addresses
- `0xa6A42798bdcc8e0DaD8432146cbD5DBE666C6A02` — collaborateESCO, goldengoat, pitchforks (factory.js)

## Recommendations

1. **Scrub addresses #1-#4** from git history (these are clearly personal wallet addresses)
2. **Review #5-#7** with Nick to confirm if personal or placeholder
3. **Replace hardcoded personal addresses** with environment variables before scrubbing
4. **Leave contract addresses** as-is (they're public by design)
5. **Consider .gitignore** for `attached_assets/` directories containing pasted API responses
6. **Tools:** Use `git filter-repo` or BFG Repo Cleaner for history rewrite
