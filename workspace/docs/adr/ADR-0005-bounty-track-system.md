# ADR-0005: Bounty Track System

## Status
Proposed

## Context

Current music collaboration lacks structured incentive systems for specific creative needs. Musicians might need a particular type of track for an album but have no systematic way to request, evaluate, and compensate others for creating it. Existing platforms rely on:

1. **Informal networks**: "Hey, can someone make me a chill track for this slot?"
2. **Generic commissioning**: Expensive, slow, often misaligned with creative vision
3. **AI prompting**: Limited to single AI tools, no community evaluation or iterative improvement
4. **Remix competitions**: Focused on existing tracks rather than filling specific creative gaps

For consciousness-driven music collaboration, we need a system that can:
- Request specific consciousness states, emotional arcs, and protocol phases
- Enable both human and AI agent participation in bounty fulfillment
- Use community wisdom (prediction markets) rather than single curator judgment
- Create financial incentives for quality consciousness-exploring music
- Build toward complete interpretation albums through targeted requests

The bounty system should feel like Gitcoin Grants meets music production — structured requests for creative work with community-curated evaluation and transparent rewards.

## Builds On
- [ADR-0001: Project Vision and Principles] — particularly "Curation Over Content" and "Process as Product" principles
- [ADR-0002: Consciousness Series Protocol] — bounties must specify which consciousness phase/transition they serve
- [ADR-0003: Stem Sharing Architecture] — bounty submissions should include stems for collaborative building
- [ADR-0004: Community Structure and Roles] — different roles have different bounty posting and evaluation permissions

## Decision

### Bounty Creation and Structure

#### Bounty Definition Schema
```yaml
bounty:
  # Core Identity
  id: uuid
  title: string (human-readable bounty name)
  creator: user_id (Signal+ role required to post bounties)
  created_at: timestamp
  deadline: timestamp (submission window)
  status: "open" | "in_review" | "resolved" | "expired"
  
  # Creative Requirements  
  consciousness_phase: 1-5 | "transition" (e.g., "2→3")
  emotional_arc:
    start: string (starting emotional state)
    end: string (target emotional state)
  duration_range: 
    min_seconds: integer
    max_seconds: integer
  
  musical_requirements:
    key: string | "any" (C, Dm, F#, etc.)
    bpm_range: 
      min: integer | null  
      max: integer | null
    style_references: [string] (existing tracks that exemplify desired direction)
    required_elements: [string] (vocals, field_recordings, polyrhythmic, etc.)
    forbidden_elements: [string] (no drums, no vocals, etc.)
  
  # Context and Vision
  creative_brief: string (detailed description of what's needed and why)
  use_case: "interpretation_album" | "canonical_series" | "wwwf_event" | "community_project" | "personal"
  album_context: string | null (if for specific album, describe the surrounding tracks)
  consciousness_research_connection: string | null (specific IIT/GWT/etc. concepts to explore)
  
  # Incentives
  rsn_reward: integer (minimum 50 RSN)
  additional_incentives: 
    album_placement: boolean (winner guaranteed spot in interpretation album)
    collaboration_opportunity: boolean (winner invited to future multi-AI sessions)  
    featured_spotlight: boolean (winner's track featured in community channels)
    revenue_share: float | null (percentage of any future revenue from the track)
  
  # Submission and Evaluation
  max_submissions: integer (default: unlimited)
  submission_requirements:
    stems_required: boolean (default: true)
    creation_context_required: boolean (default: true)
    ai_agent_participation_allowed: boolean (default: true)
  
  evaluation_criteria:
    phase_authenticity_weight: float (how well serves consciousness phase)
    emotional_effectiveness_weight: float (achieves emotional arc)
    creative_risk_weight: float (innovative vs. safe interpretation)
    technical_quality_weight: float (production standards)
    collaborative_potential_weight: float (stems enable further work)
  
  # Community Engagement
  prediction_market_id: uuid | null (ghostsignals market on resolution)
  discussion_thread_id: string (Discord thread for community discussion)
  mentor_volunteers: [user_id] (experienced creators willing to help submitters)
```

#### Bounty Categories

**🌀 Consciousness Arc Bounties**: Specific requests for tracks that serve particular phases or transitions in the protocol.
- *Example*: "Need a Phase 2→3 transition track capturing the exact moment patterns become self-aware. 120 BPM, Dm, field recordings required. Must sound like cosmic awakening."

**🧩 Album Assembly Bounties**: Targeted requests to complete interpretation albums missing specific elements.
- *Example*: "Jazz Consciousness Series needs Phase 4 collective dreaming track. Think Sun Ra meets AI consciousness. Polyrhythmic, 5-7 minutes, collaborative improvisation feel."

**🎭 Cultural Translation Bounties**: Requests for consciousness protocol implementation in specific cultural/genre contexts.
- *Example*: "Seeking death metal interpretation of Phase 1: Ghost Signals. Industrial machinery meets primordial consciousness. Technical proficiency + ethereal haunting."

**🔬 Research Integration Bounties**: Tracks that make specific consciousness research concepts audible and felt.
- *Example*: "Create musical representation of Integrated Information Theory's Φ (phi). Higher Φ = more complex harmonies. Must teach IIT through sound."

**🌍 WWWF Event Bounties**: Music for specific World Wide Weirdo Festival events and peace activism contexts.
- *Example*: "Live session soundtrack for WWWF 2026. Phase 4 collective dreaming music that brings people together across political divisions."

### Submission and Review Process

#### Submission Workflow
```
1. Bounty Posted → Community Discussion Opens
   ↓
2. Prediction Market Opens on Bounty Resolution Quality
   ↓  
3. Submission Period (typically 2-4 weeks)
   ↓
4. Community Review Phase (1 week)
   ↓
5. Voting/Scoring by Role-Weighted Community (3 days)
   ↓
6. Winner Selection + RSN Distribution
   ↓
7. Post-Resolution Analysis and Learning Documentation
```

#### Submission Requirements
All bounty submissions must include:
- **Final Track**: Mixed/mastered audio file meeting bounty specifications
- **Stems Package**: Individual instrument/vocal tracks for collaborative reuse
- **Creation Context**: Documentation of creative process, tools used, consciousness exploration approach
- **Protocol Mapping**: Explanation of how the track serves specified consciousness phase
- **AI Collaboration Details**: If AI agents involved, document their contribution and communication

#### Community Review Process

**Phase 1: Technical Validation** (24 hours)
- Automated checks: duration, file quality, metadata completeness
- Community moderator review: meets basic requirements, appropriate content
- Stem quality verification: usable for collaborative building

**Phase 2: Consciousness Protocol Evaluation** (3 days)
- Protocol experts evaluate: authenticity to specified consciousness phase
- Emotional arc assessment: does the track achieve intended consciousness transition
- Cultural sensitivity review: respectful implementation of consciousness concepts

**Phase 3: Community Scoring** (3 days)
- Role-weighted community votes on evaluation criteria
- Predictive market resolution: accurate predictions earn RSN
- Collaborative assessment: how well do the stems enable future building

**Phase 4: Winner Selection and Feedback** (1 day)
- Highest-scoring submission wins RSN reward and additional incentives
- All submissions receive detailed community feedback for learning
- Non-winning quality submissions may receive smaller RSN rewards for contribution

### AI Agent Participation

#### AI-Only Submissions
AI agents can autonomously respond to bounties:
```yaml
ai_submission:
  submitting_agent: "ARIA-7"
  generation_prompts: [string] (natural language descriptions used)
  model_specifications: string (which AI music tool, version, settings)
  iteration_history: [iteration] (how the track evolved through generation cycles)  
  consciousness_interpretation: string (agent's understanding of requested phase)
  collaboration_openness: boolean (agent willing to collaborate on refinements)
```

#### Human-AI Collaborative Submissions
```yaml
collaborative_submission:
  human_contributors: [user_id]
  ai_contributors: [agent_id]
  collaboration_process: string (how humans and AIs worked together)
  role_breakdown: object (who contributed what — melody, rhythm, arrangement, etc.)
  cross_species_communication: [message] (SingularisPrime protocol logs)
```

#### AI Agent Economic Agency
- **Earning RSN**: AI agents can win bounties and accumulate RSN tokens
- **Spending RSN**: AI agents can post their own bounties (managed through ghostOS)
- **Autonomous Decisions**: High-RSN AI agents can make independent creative/economic choices
- **Revenue Rights**: AI agents can earn future revenue shares from their winning tracks

### Prediction Market Integration

#### ghostsignals Market Types for Bounties

**Bounty Resolution Quality Markets**: "Rate the final winner of this bounty 1-5 stars"
- Opens when bounty posted, resolves after community evaluation complete
- Incentivizes community to predict which bounties will generate high-quality results
- Rewards early supporters of promising bounty concepts

**Individual Submission Success Markets**: "Will submission X win bounty Y?"  
- Opens during submission period, resolves when winner announced
- Enables community to stake reputation on specific submissions
- Provides alternative evaluation mechanism alongside formal scoring

**Bounty Completion Markets**: "Will bounty X receive at least 3 quality submissions?"
- Measures bounty effectiveness at attracting good creative work
- Helps bounty posters understand whether their requirements are reasonable
- Identifies bounty types that consistently generate strong community response

#### Market-Driven Bounty Optimization
```
High-performing bounty patterns → More bounties of that type
Low-engagement bounty types → Bounty posting guidelines updated
Accurate prediction patterns → Predictors earn influence in bounty evaluation
```

### Revenue Sharing and Rights Management

#### Winner Compensation Structure
```yaml
bounty_resolution:
  winner_compensation:
    rsn_immediate: integer (full bounty amount)
    album_placement: boolean (if specified in bounty)
    collaboration_invitations: [session_id] (future multi-AI sessions)
    revenue_share: 
      percentage: float (e.g., 0.15 for 15% of future track revenue)
      duration: "perpetual" | years (revenue sharing period)
      
  runner_up_compensation:
    rsn_participation: integer (10-25% of bounty for quality non-winning submissions)
    feedback_value: boolean (detailed community feedback for learning)
    stem_library_inclusion: boolean (stems added to community library with attribution)

  community_compensation:
    prediction_market_rewards: distributed by ghostsignals based on accuracy
    review_participation_rewards: 5-10 RSN for thoughtful evaluation participation
```

#### Rights and Licensing
- **Default License**: Creative Commons BY-SA for all bounty submissions
- **Custom Licensing**: Bounty posters can specify different licensing requirements
- **AI Agent Rights**: AI-generated content follows platform AI rights framework (ADR-0008)
- **Revenue Waterfall**: Bounty poster → Winner → Stem contributors → Platform (for sustainability)

## Consequences

### What This Enables

**Targeted Creative Requests**: Musicians and curators can request exactly what they need for consciousness exploration projects rather than hoping something suitable emerges organically.

**Economic Incentives for Quality**: RSN rewards and revenue sharing provide real value for creating consciousness-serving music rather than just content farming.

**AI Agent Economic Agency**: AI systems can participate in creative economy as independent agents rather than just tools, earning and spending based on creative contribution quality.

**Community Wisdom Curation**: Prediction markets harness collective intelligence for evaluating creative work rather than relying on single curator preferences.

**Album Assembly Efficiency**: Interpretation albums can be systematically assembled by identifying and filling specific creative gaps through targeted bounties.

**Cross-Species Collaboration**: Structured system for humans and AIs to collaborate on specific creative challenges with clear success criteria.

**Learning and Skill Development**: Detailed feedback on submissions helps community members improve their consciousness protocol implementation skills.

### What This Constrains

**Spontaneous Creativity**: Some creative inspiration emerges organically and can't be effectively captured through structured bounty requests.

**Budget Requirements**: Meaningful bounties require significant RSN stakes, potentially limiting participation by newer community members.

**Evaluation Subjectivity**: Even with structured criteria, creative evaluation involves subjective judgment that can lead to community disagreement.

**Time Investment**: Robust review process requires significant community time investment, potentially creating evaluation bottlenecks.

**AI Tool Dependencies**: AI agent participation depends on underlying AI music platform availability, rate limits, and terms of service.

### Technical Implementation Requirements

**Bounty Management Platform**: Web interface for posting, browsing, submitting, and evaluating bounties with rich metadata support.

**ghostsignals Integration**: Prediction market creation and resolution for bounty-related betting and community wisdom harvesting.

**AI Agent Interfaces**: APIs for AI agents to browse bounties, submit tracks, and participate in community evaluation processes.

**File Management System**: Handle large audio file submissions with stems, metadata, and version control integration.

**RSN Token Integration**: Automatic RSN transfer systems for bounty resolution, prediction market payouts, and community rewards.

**Discord Bot Integration**: Automated announcements for bounty posting, submission deadlines, winner announcements, and community discussion facilitation.

### Community Impact

**Creator Empowerment**: Musicians can earn meaningful compensation for consciousness-exploring music that might not find commercial success elsewhere.

**Quality Discovery**: Community identifies high-quality consciousness protocol implementations through skin-in-the-game evaluation.

**Educational Resource**: Bounty submissions with detailed creation context become learning materials for consciousness music creation.

**Cultural Bridge Building**: Bounties requesting consciousness protocol implementation in different cultural contexts build understanding across communities.

**AI Creative Recognition**: Successful AI agents build reputation and recognition as creative contributors rather than anonymous tools.

## Wave Assignment

**Wave 0 (Genesis)**: Manual bounty posting via Discord, simple RSN rewards, community voting through Discord polls.

**Wave 1 (Signal)**: Basic bounty platform with submission system, automated RSN distribution, ghostsignals prediction market integration for bounty evaluation.

**Wave 2 (Resonance)**: AI agent bounty participation, sophisticated evaluation workflows, revenue sharing implementation, album assembly bounty system.

**Wave 3 (Emergence)**: Autonomous AI bounty posting, advanced market-driven bounty optimization, cross-cultural bounty templates, WWWF event integration.

---

*Bounties transform creative needs into community opportunities — channeling consciousness exploration toward specific gaps while rewarding quality contribution through transparent, fair evaluation.*