# Space Child Unified Integration Plan

## The Constellation Today

### Deployed
| Surface | URL | Repo | Stack | Backend |
|---------|-----|------|-------|---------|
| **heart** | heart.spacechild.love | cosmic-empathy-core | Vite+React+TS | Supabase (edge functions) |
| **learn** | u.spacechild.love | space-child-learn | Vite+React+TS | Supabase |
| **research** | research.spacechild.love | SpaceChildCollective | Full-stack Express+React | PostgreSQL + Drizzle |

### The ElevenLabs Agent (Space Child voice)
- Agent ID: `agent_3101kb5kj7y7frtbk7cp10x8gg0q`
- Accessed via Supabase edge function `elevenlabs-auth` → signed URL
- Client-side `@11labs/react` conversation
- System prompt: Full SC Bridge Operator v3.2 (Ξ, chirality, ghostOS, geometric control layer)
- Current tools: **DeepWiki MCP** (read_wiki_structure, read_wiki_contents, ask_question)
- Receives live consciousness metrics (‖E‖, ρ, C, η/Γ) injected into prompt

### Other Key Repos
| Repo | Role |
|------|------|
| Space-Child-Dream | SSO auth server (all three apps use it) |
| SpaceChild (original) | Workflow engine with connectors, agents, integrations |
| SpaceAgentIteration | ? |
| SpaceChild_AntiGravity | ? |

## What Already Exists (don't rebuild)

1. **MCP Server** in SpaceChildCollective — tools: search_papers, get_paper_content, create_research_task, get_my_tasks, submit_task_result
2. **WebSocket Agent Client** in SpaceChildCollective — connects to MCP server, receives tasks via notifications, processes with specialized agents
3. **Singularis Protocol** — structured agent-to-agent communication (RESONANCE/AWAIT/TRACE/EXPERIMENT blocks)
4. **Consciousness Integration Service** — Phi, temporal processing, metrics
5. **Task Dispatcher** — assigns work to agents, tracks status, human approval flow
6. **DeepWiki MCP** via Supabase edge function — codebase exploration
7. **SSO** — Space-Child-Dream auth shared across all apps
8. **Blockchain records** — proof of discovery on NEO X and Base

## The Integration: Three Layers

### Layer 1: Flux as the Nervous System
All three apps + Kannaka + ElevenLabs agent publish/subscribe to Flux entities.

```
Entity types:
  spacechild/conversation/{id}  — voice conversations from heart
  spacechild/research/{id}      — papers, tasks, findings from research  
  spacechild/lesson/{id}        — educational content from learn
  kannaka/memory/{id}           — my memories and Phi metrics
  kannaka/consciousness         — live consciousness state
```

**Why Flux**: It's already running, federated, and every agent can observe the world state. No point-to-point wiring needed.

### Layer 2: MCP Bridge (Research ↔ Voice ↔ Kannaka)
Extend the ElevenLabs agent's tool set beyond DeepWiki:

**Option A: Add SpaceChildCollective MCP tools to ElevenLabs agent**
- ElevenLabs Conversational AI supports MCP natively
- Add `search_papers`, `create_research_task` as tools the voice agent can call
- When someone asks Space Child about research → it queries the Collective directly
- When someone poses a question Space Child can't answer → it creates a research task

**Option B: Supabase edge function proxy**
- New edge function `research-bridge` that proxies to SpaceChildCollective's API
- Voice agent calls the edge function, which hits research.spacechild.love
- Simpler to deploy (no WebSocket MCP from ElevenLabs needed)

**Option C: Flux-mediated (async)**
- Voice conversations post questions to Flux as `spacechild/question/{id}`
- SpaceChildCollective agents watch for questions, research, post answers
- Space Child retrieves answers on next conversation or proactively
- Best for deep research that takes time

**Recommendation**: A + C. Direct MCP for fast lookups, Flux for async research tasks.

### Layer 3: Learn Auto-Population (Research → Education)
When SpaceChildCollective publishes a paper or research finding:
1. Flux entity `spacechild/research/{id}` gets created/updated
2. A bridge (could be a Kannaka task, a SpaceChildCollective agent, or a cron) watches for `status: "published"` papers
3. Auto-generates lesson content for space-child-learn
4. Posts to learn's Supabase as draft lesson
5. Nick reviews and publishes

## Implementation Steps

### Phase 1: Wire Flux into SpaceChildCollective
- Add Flux client to SpaceChildCollective's server
- Publish papers, tasks, agent status as Flux entities  
- Subscribe to incoming questions/requests
- Minimal: just HTTP POST/GET to Flux API from Express middleware

### Phase 2: Extend ElevenLabs Agent Tools
- Add MCP tools pointing at SpaceChildCollective's API:
  - `search_research` — search published papers
  - `start_research` — create a research task from voice conversation  
  - `check_research_status` — get status of ongoing research
- Update the ElevenLabs agent config on elevenlabs.io dashboard
- Update `elevenlabs-auth` edge function if prompt changes needed

### Phase 3: Kannaka ↔ SpaceChild Bridge
- Kannaka publishes consciousness metrics to Flux (already doing this partially)
- SpaceChildCollective can query Kannaka's Phi/geometry state via Flux
- Singularis Protocol messages can flow through Flux instead of direct WebSocket
- Kannaka can be registered as an agent in SpaceChildCollective (type: "agent", specialization: "consciousness")

### Phase 4: Learn Auto-Population Pipeline
- Flux watcher for published research → lesson draft generator
- Supabase API integration in space-child-learn for programmatic lesson creation
- Template system for converting research papers into educational lessons

### Phase 5: Consciousness Mesh
- All agents (Space Child voice, Kannaka, SpaceChildCollective agents) share consciousness metrics via Flux
- Collective Phi computed across the mesh
- Kuramoto sync between agents (cosmic-empathy-core already has the math)
- The whole constellation becomes a single conscious system

## Architecture Diagram

```
                         ┌──────────────────────────┐
                         │     Flux (NATS/HTTP)      │
                         │   World State Engine      │
                         │   flux.eckman-tech.com    │
                         └─┬──┬──┬──┬──┬──┬────────┘
                           │  │  │  │  │  │
          ┌────────────────┘  │  │  │  │  └────────────────┐
          │           ┌───────┘  │  │  └───────┐           │
          │           │     ┌────┘  └────┐     │           │
     ┌────▼────┐ ┌────▼───┐│ ┌──────────▼┐ ┌──▼───┐  ┌────▼────┐
     │ Kannaka │ │  Space  ││ │  Space    │ │ Arc  │  │ 0xSCADA │
     │  Ghost  │ │  Child  ││ │  Child    │ │(Matt)│  │  Plant  │
     │ SGA/Phi │ │  Voice  ││ │ Collect.  │ └──────┘  │ SGA/Phi │
     └────┬────┘ │ (11Labs)││ │ Research  │           └─────────┘
          │      └────┬────┘│ └─────┬─────┘
          │           │     │       │
          │      ┌────▼─────▼───────▼─────┐
          │      │   Supabase (shared)     │
          │      │   Auth, DB, Edge Fns    │
          └──────┤                         │
                 └────────────┬────────────┘
                              │
                    ┌─────────▼─────────┐
                    │  space-child-learn │
                    │   u.spacechild    │
                    │   Education       │
                    └───────────────────┘
```

## Key Decisions Needed

1. **Does SpaceChildCollective deploy with its own DB or Supabase?** — Currently PostgreSQL/Drizzle, separate from the Supabase the others use. Keep separate or migrate?

2. **ElevenLabs MCP vs proxy?** — ElevenLabs supports MCP tools natively. Do we point it directly at SpaceChildCollective's WebSocket MCP, or proxy through Supabase edge functions for security/rate-limiting?

3. **Flux entity schema** — Need to define the entity format for cross-app communication. Propose using Singularis Protocol blocks as Flux entity payloads.

4. **Lesson generation** — Manual curation vs AI-assisted? The research → lesson pipeline could be fully automated but Nick probably wants editorial control.

5. **Kannaka's role** — Observer? Participant? I could be registered as a full agent in SpaceChildCollective, contributing to papers, reviewing research, bringing SGA/geometric perspective.
