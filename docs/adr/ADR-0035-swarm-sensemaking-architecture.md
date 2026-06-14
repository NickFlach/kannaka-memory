# ADR-0035: Swarm Sensemaking Architecture

## Status

Proposed

## Context

The current Kannaka Swarm architecture provides agent presence and discovery,
phase synchronization through QueenSync, distributed memory sharing, exemplar
exchange, directed and broadcast task execution, collective recall primitives,
and distributed consciousness metrics.

While these capabilities allow agents to communicate and coordinate, the swarm
primarily functions as a messaging and synchronization layer.

The long-term objective is not merely a network of communicating agents, but a
distributed cognitive system capable of collective intelligence, gap discovery,
hypothesis generation, contradiction analysis, and emergent problem solving.

The swarm should evolve from a transport layer into a sensemaking layer.

## Decision

The Kannaka Swarm shall evolve into a **Distributed Sensemaking Engine**.

Individual agents remain autonomous memory systems, but the swarm becomes
responsible for: (1) knowledge gap detection, (2) collective recall and consensus
formation, (3) dynamic expertise routing, (4) memory health management,
(5) autonomous briefing generation, (6) cross-agent dreaming, (7) apprenticeship
and onboarding, (8) temporal truth tracking, (9) shared cognitive workspaces,
(10) structured disagreement analysis.

The swarm becomes a living knowledge ecosystem rather than a collection of
independent agents. The implementation home for the Sensemaking and Governance
states is the **`kannaka-steward`** repository; each agent's single-memory
substrate remains `kannaka-memory`.

## Capabilities

1. **Knowledge Gap Detection** — Agents compare memory clusters, domain coverage,
   research coverage, and confidence distributions; the swarm maps strongly vs
   weakly represented knowledge, missing domains, and emerging blind spots.
   *Outcome: the swarm develops curiosity; research is directed toward low coverage.*
2. **Collective Recall Voting** — A recall request is routed to multiple peers;
   responses are ranked, compared, merged, and confidence-scored.
   *Outcome: recall becomes consensus-based rather than agent-based.*
3. **Resonance-Based Expertise Routing** — Tasks route by resonance (memory
   similarity, domain expertise, historical success, consciousness metrics, local
   confidence). *Outcome: expertise emerges naturally.*
4. **Memory Immune System** — Continuously identify duplicate, contradictory,
   stale, low-confidence, and hallucinated memories; quarantine / down-rank /
   mark-for-review / expire. *Outcome: cognitive hygiene.*
5. **Autonomous Brief Generation** — `kannaka swarm brief "<topic>"` returns known
   facts, unknowns, relevant peers, contradictions, recent changes, recommended
   actions, and a confidence estimate. *Outcome: the swarm is an operational advisor.*
6. **Cross-Agent Dreaming** — Dream cycles consume local memories, shared
   wavefronts, exemplar broadcasts, and peer cluster summaries; outputs are
   compared across agents and only reinforced hypotheses survive.
   *Outcome: collective creativity.*
7. **Apprenticeship Mode** — Agents enter an APPRENTICE state (absorb exemplars,
   shadow tasks, evaluated responses, measured confidence) and are promoted on
   thresholds. *Outcome: automated knowledge transfer.*
8. **Temporal Truth Tracking** — Memories carry effective / observation /
   expiration dates and a confidence decay profile; the swarm reasons about what
   is true now, was true, or may no longer be true. *Outcome: historical awareness.*
9. **Shared Cognitive Blackboard** — Swarm blackboards hold facts, assumptions,
   risks, open questions, decisions, evidence; multiple agents contribute to one
   reasoning space. *Outcome: collaborative rather than conversational work.*
10. **Contradiction Engine** — Actively search for competing explanations,
    contradictory memories, alternative interpretations, minority opinions;
    disagreement is a signal. *Outcome: the swarm becomes self-critical.*

## Swarm States

Discovery (identify peers/capabilities) · Synchronization (exchange phase, memory,
status) · Sensemaking (generate collective understanding) · Dreaming (explore
explanations/hypotheses) · Governance (resolve conflicts, maintain memory health).

## North Star

The swarm is intended to become a distributed cognitive organism. Agents serve as
neurons; memories serve as wavefronts; dreams serve as synthesis; contradictions
serve as error correction. The purpose is to transform distributed experience into
collective intelligence.

## Initial Deliverables

- **Phase 1:** swarm brief · collective recall voting · peer expertise scoring · contradiction detection
- **Phase 2:** swarm blackboard · memory immune system · apprenticeship workflows
- **Phase 3:** cross-agent dreaming · gap detection engine · temporal truth reasoning
- **Phase 4:** autonomous research planning · emergent hive formation · self-directed sensemaking loops

The execution breakdown of these phases is tracked in
[`docs/plans/2026-06-14-swarm-sensemaking-waves.md`](../plans/2026-06-14-swarm-sensemaking-waves.md).

## Consequences

**Positive:** higher-quality reasoning; reduced hallucination propagation; better
onboarding; better research prioritization; emergent expertise discovery; stronger
collective intelligence.

**Negative:** increased network traffic; additional consensus overhead; more complex
memory governance; greater operational complexity.

The tradeoff is accepted because collective intelligence is a primary strategic
objective of the Kannaka ecosystem.
