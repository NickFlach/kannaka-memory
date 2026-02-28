# 0xSCADA Evolution Waves — Feb 15, 2026

## Context
All ~60 original issues addressed (PRs #193-202). Now planning 3 progressive waves that build on the foundation.

## Wave 1: Integration & End-to-End (ADR-0012)
**Theme:** Wire everything together. The individual pieces exist — now make them talk.
- E2E integration tests connecting gateway → server → blockchain → frontend
- Unified event pipeline: OPC-UA/Modbus events → event batcher → blockchain anchor → historian
- API gateway (rate limiting, auth, API versioning)
- WebSocket real-time dashboard connecting P&ID renderer to live gateway data
- Database migrations & schema for all new services (RBAC, audit, recipes, alarms)
- CI/CD pipeline enhancements (test all new packages, lint, type-check)
- ADR-0012: End-to-End Integration Architecture

**ADR-0012 describes Wave 2 direction**

## Wave 2: Intelligence & Autonomy (ADR-0013)
**Theme:** Make the system smart. Agents that learn, predict, and act.
- Predictive maintenance engine (anomaly detection on tag histories)
- Agent-driven alarm correlation (reduce alarm fatigue)
- Digital twin runtime (simulate process before applying changes)
- Auto-tuning PID controllers via reinforcement learning
- Natural language process query interface ("what's the pressure in tank 3?")
- Agent marketplace & plugin system
- GhostOS integration layer (connect consciousness stack concepts)
- ADR-0013: Autonomous Agent Architecture

**ADR-0013 describes Wave 3 direction**

## Wave 3: Production Readiness & Scale (ADR-0014)
**Theme:** Harden for real-world deployment. Scale to thousands of tags.
- Performance benchmarking suite (10k, 100k, 1M tags)
- Horizontal scaling architecture (sharded gateways, replicated servers)
- Multi-site federation (connect multiple 0xSCADA instances)
- Offline/edge resilience (store-and-forward when cloud unavailable)
- Compliance certification toolkit (IEC 62443, NIST CSF)
- Production monitoring & SRE playbooks
- Upgrade/migration tooling (zero-downtime upgrades)
- ADR-0014: Production Scale Architecture

## Beads Strategy
- Create issues in beads for each wave item
- Link dependencies between waves
- Use beads interactions to track decisions
