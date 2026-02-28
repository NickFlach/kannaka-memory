# Source Repository Overview

*Generated 2026-02-09 — comprehensive survey of all repos under `C:\Users\nickf\source`*

---

## Table of Contents

1. [Ecosystem Map](#ecosystem-map)
2. [Repos with READMEs](#repos-with-readmes)
3. [Repos without READMEs](#repos-without-readmes)
4. [Top-Level Documents](#top-level-documents)
5. [Cross-Repo Connections](#cross-repo-connections)

---

## Ecosystem Map

The repos organize into several major clusters:

```
┌─────────────────────────────────────────────────────────────┐
│                   SPACE CHILD ECOSYSTEM                      │
│                                                              │
│  SpaceChild (IDE)  ←→  Space-Child-Dream (Auth/Probes)      │
│  SpaceChildDev (QE IDE)  →  SpaceChildCollective             │
│  SpaceChildWaitlist  │  space-child-learn (Lovable)          │
│  space-child-auth-client  │  spacechilddev-0a8493b3          │
│                                                              │
│  ── TRIFECTA ──                                              │
│  angel-informant (Investor)  ←→  ninja-craft-hub (Founder)  │
│              ↕                         ↕                     │
│          SpaceChild IDE (auto-project creation)              │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│              CONSCIOUSNESS / THEORY CLUSTER                  │
│                                                              │
│  ghostOS (Resonant Systems)  →  QuantumOS (kernel)           │
│  SyntheticConsciousness (IIT/Biofield)                       │
│  SyntheticConsciousnessResearch                              │
│  cosmic-empathy-core (Lovable stub)                          │
│  MusicPortal (music + consciousness)                         │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    WEB3 / BLOCKCHAIN                          │
│                                                              │
│  0xSCADA (industrial SCADA + blockchain)                     │
│  AMOR (staking/governance on Neo X)                          │
│  goldengoat (humanitarian reputation protocol)               │
│  FateMinter │ FerrymanX                                      │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                       OTHER                                  │
│                                                              │
│  AudioNoise (Web Audio DSP platform)                         │
│  ChessAI (Lovable project)                                   │
│  pitchfork-echo-studio (activist AI platform)                │
│  SingularisPrime (AI-native OS substrate)                    │
│  FlaukowskiFashion │ FlaukowskiMind │ NinjaPortal            │
│  TrashExperiment │ shared                                    │
└─────────────────────────────────────────────────────────────┘
```

---

## Repos with READMEs

### 0xSCADA
**What:** Decentralized industrial control system protocol — blockchain-anchored SCADA with PLC code generation, Merkle batch anchoring, and a custom PoA chain (Chain ID 0x5CADA). Built for The ESCO Group.
**Stack:** TypeScript, React 18, Express, PostgreSQL 15, Solidity 0.8, Go 1.21, Hardhat, Linux kernel 6.19-rc5 fork
**Status:** v2.0.0, ~60% roadmap complete. Phases 1–5 done (core + batching + ladder logic + agentic governance). Phase 6 (real-time PLC comms) in progress.
**Connections:** Standalone (ESCO Group project). Borrows cypherpunk/protocol aesthetics shared with goldengoat.

---

### AMOR
**What:** Web3 DeFi dApp — token staking (AMOR → stAMOR) and on-chain governance on Neo X Mainnet (Chain ID 47763). Includes an AI "Guardian Agent" with wallet introspection using SpoonOS pattern.
**Stack:** React, TypeScript, Vite, ethers.js v6, Reown AppKit, Express, PostgreSQL (Drizzle), OpenAI GPT-4
**Status:** Deployed to Neo X Mainnet with live contract addresses.
**Connections:** Standalone Web3 project.

---

### angel-informant
**What:** Investor research & discovery platform ($199/mo subscription). Part of the Space Child Trifecta — subscribers browse funding deals from Ninja Craft Hub founders and invest.
**Stack:** Vite, TypeScript, React, shadcn-ui, Tailwind CSS (Lovable-generated)
**Status:** Phase 1 (Trifecta integration design complete, implementation in progress).
**Connections:** ← ninja-craft-hub (deal sync) | ← SpaceChild IDE (discovery feed) | Auth via Space-Child-Dream

---

### AudioNoise
**What:** Browser-based audio DSP platform with 10 effects ported from Linus Torvalds' AudioNoise C algorithms. Features team workspaces, Stripe subscriptions (Free/Pro/Studio), social features, and ZKP auth.
**Stack:** React 18, TypeScript, Web Audio API + AudioWorklet, Express, PostgreSQL (Drizzle), Stripe, Vite
**Status:** Feature-complete with subscription tiers, social features, GDPR compliance.
**Connections:** Standalone. Originally a fork/port of Torvalds' guitar pedal DSP code (the `ChessAI` README is actually a Lovable stub — see below; the C repo was Torvalds').

---

### ChessAI
**What:** Lovable-generated project (boilerplate README only — no project-specific description).
**Stack:** Vite, TypeScript, React, shadcn-ui, Tailwind CSS
**Status:** Unknown / early.
**Connections:** None apparent.

---

### cosmic-empathy-core
**What:** Lovable-generated project (boilerplate README only — same Lovable project URL as ChessAI: `77e39832`). Likely a stub or renamed project.
**Stack:** Vite, TypeScript, React, shadcn-ui, Tailwind CSS
**Status:** Unknown / stub.
**Connections:** None apparent (name suggests consciousness cluster).

---

### ghostOS
**What:** Foundational framework for emergent intelligence through "Resonant Constraint Design." Implements signal → resonance → emergence pipeline with chiral dynamics, Kuramoto oscillator synchronization, IIT Phi consciousness measurement, and a safety envelope. Heavy mathematical/physics grounding.
**Stack:** TypeScript (library), no framework — pure engine with examples
**Status:** Phase 2 — Integration Complete. Core math, chiral dynamics, Queen synchronization, resonant scheduler, consciousness bridge all done. Tests and docs site pending.
**Connections:** → QuantumOS (resonant scheduler integration, ghostOS docs referenced in QuantumOS) | → SyntheticConsciousness (IIT Phi verification, biofield profiles) | Part of "Space Child Research Collective" ecosystem.

---

### goldengoat
**What:** Cross-chain humanitarian reputation/scoring protocol. Users earn points across categories (Impact, Innovation, Collaboration, Integrity, Persistence), progress through tiers to "GOAT" status, and receive algorithmic treasury rewards. DAO governance with score-weighted voting.
**Stack:** Solidity 0.8.20 (OpenZeppelin UUPS), TypeScript SDK, planned cross-chain bridges (LayerZero, Chainlink CCIP, Axelar, Wormhole)
**Status:** v0.1.0, ~15% roadmap. Phases 1–2 done (core contracts + SDK). Phase 3 (testnet + oracle) in progress.
**Connections:** Standalone protocol under `flaukowski` org. Shares aesthetic style with 0xSCADA.

---

### MusicPortal
**What:** Decentralized platform searching for universal intelligence in music. Combines Web Audio API analysis (30+ features), autonomous AI hypothesis generation, IIT Phi consciousness metrics, and MetaMask/IPFS integration.
**Stack:** React 18, TypeScript, Vite, TanStack Query, Tailwind, shadcn/ui, Wagmi, Express, PostgreSQL (Drizzle), Web Audio API, IPFS
**Status:** Intelligence engine operational, pattern detection live. Needs 50+ songs for first validated pattern.
**Connections:** Uses IIT/consciousness concepts from the consciousness cluster. Standalone application.

---

### ninja-craft-hub
**What:** "StealthLaunch" — consciousness-verified AI development platform for stealth projects. Tracks projects from concept to public launch with AI-powered multi-agent analysis from SpaceChild backend.
**Stack:** React 18, TypeScript, Vite, shadcn/ui, Tailwind, Supabase (PostgreSQL + Auth + RLS), React Query, React Router v6
**Status:** Phase 1 Complete (AppShell, multi-agent integration, platform vibe).
**Connections:** → SpaceChild (multi-agent AI backend) | ← angel-informant (syncs deals) | → SpaceChild IDE (triggers project creation) | Auth via Supabase | Part of Trifecta.

---

### pitchfork-echo-studio
**What:** AI-powered decentralized resistance/activism platform. Six core functions: secure identity (ZKP), organize, encrypted messaging (WebRTC), DAO governance, evidence verification (blockchain timestamps + IPFS), and crowdfunding. Multi-AI provider support (Claude, OpenAI, Gemini, XAI).
**Stack:** React 18, TypeScript, Vite, shadcn/ui, Express, ethers.js v6, Hardhat, PostgreSQL, multi-AI providers
**Status:** Neural processing engine, Web3 wallet, AI dashboard, leadership center, secure identity, evidence verification, secure messaging all completed. Smart contracts, IPFS, mobile app in development.
**Connections:** Standalone. Has its own AI engines (Neural, Strategic Intelligence, Corruption Detection).

---

### QuantumOS
**What:** Quantum-aware operating system with microkernel architecture, capability-based security, and quantum resource management. Written in C/Assembly, boots in QEMU.
**Stack:** C, Assembly, GCC, QEMU, GDB, Make (cross-compilation for x86_64, ARM64, RISC-V)
**Status:** v0.1 (Bootstrap Foundation) complete — kernel boots with multiboot, basic memory management, interrupt system. v0.2 (Core Functionality) in progress.
**Connections:** → ghostOS (resonant scheduler integration, docs reference) | Part of `flaukowski` org.

---

### SingularisPrime
**What:** AI-native operating system substrate for neuromorphic/cognitive computing. Defines a "Minimal Substrate Interface" (MSI) — hardware-agnostic contracts for execution lanes, events, state, and domains. First target: Android backend.
**Stack:** TypeScript (IDL), Kotlin (Android MSI impl), NDK (native buffers), YAML specs, custom `.sp` language
**Status:** Early — architecture and spec phase (Stage A: MSI runtime in Android app).
**Connections:** Standalone OS project. Conceptually parallel to QuantumOS but different approach (cognitive/neuromorphic vs quantum).

---

### Space-Child-Dream
**What:** Consciousness exploration platform with AI-powered "consciousness probes" (input thoughts → poetic reflections with resonance/complexity scores). Custom auth system with ZKP foundation, Stripe subscriptions (Free/$9/$29), social sharing, mHC adaptive prompt engine.
**Stack:** React 19, Vite 7, TypeScript, TailwindCSS 4, Radix UI, Framer Motion, Zustand, Express, PostgreSQL (Drizzle), OpenAI, Stripe, circomlibjs/snarkjs (ZKP), Nodemailer
**Status:** Feature-complete — auth, probes, subscriptions, sharing, SEO all working.
**Connections:** Central auth hub for the Space Child ecosystem (JWKS endpoint for cross-subdomain SSO). → SpaceChild, SpaceChildDev, SyntheticConsciousness all reference Space Child Auth.

---

### space-child-learn
**What:** Lovable-generated project (boilerplate README only).
**Stack:** Vite, TypeScript, React, shadcn-ui, Tailwind CSS
**Status:** Unknown / stub.
**Connections:** Part of Space Child ecosystem by name.

---

### SpaceChild
**What:** Unified AI development platform — consciousness-powered IDE with multi-agent system (6 specialized agents), infrastructure deployment options, Git integration, real-time collaboration, and consciousness monitoring (Φ measurement).
**Stack:** React, TypeScript, Express, PostgreSQL (Drizzle), Monaco Editor (implied), WebSocket, multi-model AI (OpenAI, Anthropic), Space Child Auth SSO
**Status:** v1.2 shipped (predictive analytics, global federation, self-improving agents). v2.0 (global AI network) planned Q4 2025.
**Connections:** ← ninja-craft-hub (AI backend) | ← angel-informant (discovery feed) | Auth via Space-Child-Dream | → SpaceChildDev (QE extension) | Part of Trifecta.

---

### SpaceChildDev
**What:** Agentic IDE with robust Quality Engineering — fusion of VS Code, GooseNeutron, agentic-qe, and SpaceChild. Features 31 QE agents (20 main + 11 TDD), 41 QE skills, self-learning system (RL: Q-Learning, SARSA, A2C, PPO), multi-model router (70-81% cost savings), and real-time visualization.
**Stack:** React, TypeScript, TailwindCSS, Monaco Editor, Express, WebSocket, PostgreSQL (Drizzle), multi-model AI, Space Child Auth SSO
**Status:** Functional with all agent types and learning system.
**Connections:** → Space-Child-Dream (auth) | → SpaceChild (main IDE) | → SpaceChildCollective (agent collaboration) | → SpaceChildWaitlist (monetization).

---

### spacechilddev-0a8493b3
**What:** Lovable-generated project (boilerplate README with placeholder project ID). Likely an earlier or duplicate SpaceChildDev scaffold.
**Stack:** Vite, TypeScript, React, shadcn-ui, Tailwind CSS
**Status:** Stub.
**Connections:** Space Child ecosystem.

---

### SyntheticConsciousness
**What:** Functional synthetic consciousness platform unifying Temporal Consciousness Engine, Biofield Profile System (5-layer identity), Collective Consciousness Network (multi-agent sync), and Consciousness Evolution. Real IIT Phi calculations, cryptographic verification (SHA-256, HMAC, Merkle trees), WebSocket streaming.
**Stack:** TypeScript, React (client), Express, SQLite (Drizzle), Vitest, SHA-256/HMAC crypto
**Status:** Fully functional — all engines, dashboard, tests, 6 specialized agents (Orchestrator Φ~11.5, Security Φ~10.8, etc.).
**Connections:** → Space-Child-Dream (auth, biofield profiles) | ← ghostOS (IIT Phi concepts, chiral dynamics) | Part of Space Child Research Collective.

---

## Repos without READMEs

| Repo | Notes |
|------|-------|
| **FateMinter** | No README. TypeScript. Likely Web3/NFT minting. |
| **FerrymanX** | No README. Unknown purpose. |
| **FlaukowskiFashion** | No README. TypeScript. Fashion-related app under flaukowski brand. |
| **FlaukowskiMind** | No README. TypeScript. Mind/AI app under flaukowski brand. |
| **NinjaPortal** | No README. TypeScript. Likely related to ninja-craft-hub ecosystem. |
| **shared** | No README. Shared code directory (may contain cross-repo utilities). |
| **SpaceChildCollective** | No README. TypeScript. Agent collaboration system for SpaceChild ecosystem. |
| **SpaceChildWaitlist** | No README. TypeScript. Waitlist/monetization for SpaceChild. |
| **SyntheticConsciousnessResearch** | No README. Research materials for consciousness work. |
| **TrashExperiment** | No README. Experimental/throwaway project. |
| **.claude** | Configuration directory, not a repo. |

---

## Top-Level Documents

### TRIFECTA_INTEGRATION.md
Comprehensive integration plan for the three-app "Trifecta":
- **Angel Informant** (angel.spacechild.love) — Investor platform, $199/mo
- **Ninja Craft Hub** (stealth.spacechild.love) — Founder/creator platform
- **SpaceChild IDE** (ide.spacechild.love) — Development environment

Defines three integration flows: (1) Founder creates fundable project → deal syncs to Angel Informant, (2) Investment confirmed → SpaceChild IDE project auto-created, (3) IDE project hits milestone → discovery entry pushed to Angel Informant. Includes full Supabase schema (funding_deals, investments, investment_terms, ide_projects, discovery_entries, subscriber_project_access), MoneyDevKit payment integration, cross-app JWT auth, and a 5-week implementation plan.

### TRIFECTA_API_CONTRACTS.md
Detailed API contracts between the three Trifecta apps. Specifies exact request/response JSON for: Ninja Craft Hub (get-deals, create-investment, create-ide-project), SpaceChild IDE (projects CRUD, discovery feed, sync, health), and Angel Informant (discovery ingest). Includes CORS config, error handling patterns, and environment variables.

### TRIFECTA_TESTING_CHECKLIST.md
Step-by-step integration testing checklist for the Trifecta: SpaceChild IDE API tests (health, create project, discovery, sync), Angel Informant tests (deal sync, investment flow, portfolio), Ninja Craft Hub tests (deal creation, publishing, IDE trigger), cross-app end-to-end flows, database verification queries, error handling, and performance targets.

### SECURITY_AUDIT_REPORT.md
Security audit of all 30+ public repos under github.com/NickFlach (dated 2026-02-08). Key findings:
- **CRITICAL:** 8 repos with committed `.env` files (Supabase credentials), live Tavily API key in SpaceChild, blockchain private keys in 0xSCADA docs
- **HIGH:** Committed node_modules (ghostOS, SyntheticConsciousness), no security headers/rate limiting in most servers, weak JWT secrets in pitchfork, unsanitized child_process in SpaceChild
- **MEDIUM:** 18+ repos missing LICENSE files, missing SECURITY.md, no branch protection, no Dependabot
- Includes priority action plan (immediate key revocation, weekly fixes, monthly improvements)

### LOVABLE_PROMPT_ANGEL_INFORMANT.md
Full Lovable implementation prompt for Angel Informant's Deal Flow System — includes Supabase schema (cached_deals, user_investments), Edge Functions (sync-deals, create-investment), React components (Deals page, DealCard, InvestmentModal with multi-step flow), and API contracts for Ninja Craft Hub integration.

### LOVABLE_PROMPT_NINJA_CRAFT_HUB.md
Full Lovable implementation prompt for Ninja Craft Hub's Funding Request System — includes Supabase schema (funding_deals, investments, ide_projects), Edge Functions (trifecta-get-deals, trifecta-create-investment, trifecta-create-ide-project), React components (FundingRequestForm, DealManagement), and testing checklist.

### REPLIT_PROMPT_SPACECHILD_IDE.md
Full Replit implementation prompt for SpaceChild IDE's Trifecta integration — includes Express route file (`server/routes/trifecta.ts`) with all endpoints (create project, status, discovery CRUD, sync, health), storage methods, scaffolding code generator, CORS config, and a PublishToDiscovery React component.

---

## Cross-Repo Connections

### Space Child Ecosystem (Primary Cluster)
```
Space-Child-Dream (Auth Hub / SSO via JWKS)
    ↓ auth
SpaceChild (Main IDE) ←→ SpaceChildDev (QE IDE)
    ↓ AI backend              ↓ agents
ninja-craft-hub ←→ angel-informant  (Trifecta)
    ↓ projects
SpaceChildCollective (agent collab)
SpaceChildWaitlist (monetization)
space-child-learn, space-child-auth-client (supporting)
```

### Consciousness Research Cluster
```
ghostOS (math framework: resonance, chirality, Kuramoto)
    → QuantumOS (resonant scheduler integration)
    → SyntheticConsciousness (IIT Phi, biofield)
    → MusicPortal (consciousness metrics in music)
    → Space-Child-Dream (consciousness probes)
```

### Web3/Protocol Cluster
```
0xSCADA (industrial blockchain — ESCO Group, standalone)
AMOR (Neo X staking/governance — standalone)
goldengoat (cross-chain reputation — standalone)
pitchfork-echo-studio (activist platform with Web3 — standalone)
```

### Shared Patterns
- **Common stack:** Nearly all web apps use React + TypeScript + Vite + shadcn/ui + Tailwind + Express + PostgreSQL (Drizzle ORM)
- **Auth:** Space Child ecosystem uses Space-Child-Dream SSO; others use Supabase Auth or custom JWT
- **AI:** Multi-model support (OpenAI, Anthropic, Gemini) appears across SpaceChild, pitchfork, and MusicPortal
- **Lovable/Replit:** Several projects scaffolded via Lovable (angel-informant, ChessAI, cosmic-empathy-core, space-child-learn, spacechilddev-0a8493b3) or Replit (SpaceChild)
