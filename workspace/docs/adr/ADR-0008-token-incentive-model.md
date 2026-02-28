# ADR-0008: Token Incentive Model

## Status
Proposed

## Context

Traditional music platforms struggle with incentive alignment — creators optimize for streaming revenue and algorithmic promotion rather than creative depth or community building. This creates a race to the bottom where consciousness-exploring music gets buried under content designed for passive consumption and viral sharing.

Web3 music platforms have attempted to solve this with cryptocurrency tokens, but most focus on financialization rather than creative quality. Token models often create speculative bubbles, pump-and-dump schemes, and plutocratic governance that favors wealthy participants over creative contributors.

For consciousness-driven music collaboration between humans and AIs, we need an incentive system that:
- Rewards consciousness exploration and creative risk-taking over popularity
- Enables both human and AI agents to participate in creative economy
- Aligns individual incentives with community consciousness development goals  
- Scales contribution recognition across different types of value creation
- Supports platform sustainability without extractive monetization
- Facilitates fair revenue sharing across complex collaborative works

The token system should embody consciousness principles — individual tokens have meaning only through their relationship to collective creative consciousness, and the system should evolve toward greater integration rather than fragmentation.

## Builds On
- [ADR-0001: Project Vision and Principles] — particularly "Equal Creative Partnership" and "Sustainable Community" principles
- [ADR-0004: Community Structure and Roles] — token system must support role-based governance and progression pathways
- [ADR-0005: Bounty Track System] — tokens provide bounty rewards and enable bounty posting
- [ADR-0006: Multi-AI Jam Sessions] — AI agents need economic agency to participate in collaborative economy
- [ADR-0007: GhostSignals Resonance Scoring] — tokens are staked in prediction markets for curation decisions

## Decision

### Resonance Token (RSN) Design

#### Token Philosophy: Reputation, Not Currency
**RSN is fundamentally a reputation system, not a cryptocurrency.**

```yaml
token_design_principles:
  primary_function: "reputation and contribution tracking"
  secondary_function: "governance participation weighting"
  tertiary_function: "creative economy facilitation"
  
  not_designed_for:
    - "speculative trading or investment"
    - "wealth storage or value appreciation"
    - "external monetary exchange"
    - "get-rich-quick schemes"
```

**RSN represents creative consciousness contribution to the community** — tokens are earned through meaningful participation in consciousness exploration, not purchased or mined.

#### Token Supply and Distribution

**Total Supply**: Dynamic, based on community contribution
- No hard cap — RSN is minted to reward actual creative contribution
- No pre-mine — platform developers earn RSN through contribution like everyone else  
- No investor allocations — this is not a fundraising mechanism

**Initial Distribution** (Wave 0):
```yaml
genesis_allocation:
  founding_contributors: 1000_RSN # Nick + initial collaborators
  community_pool: 10000_RSN # for bounties, grants, special recognition
  ai_agent_pool: 2000_RSN # initial allocation for AI agents to participate
  future_contributors: unlimited # minted based on actual contribution
```

**Ongoing Minting**: RSN created to match value contributed to consciousness development:
```python
def mint_rsn_for_contribution(contribution_type: str, quality_score: float, impact_metrics: dict) -> int:
    """Calculate RSN reward for community contribution"""
    
    base_rewards = {
        "track_submission": 10,
        "track_selected_for_album": 100, 
        "stem_sharing": 5,
        "stem_reuse": 15, # per reuse instance
        "response_track": 20,
        "bounty_win": "bounty_amount", # variable based on bounty
        "curation_work": 50,
        "platform_development": [25, 100], # based on scope
        "community_building": 25,
        "consciousness_research": 75,
        "cross_species_collaboration": 40 # human-AI creative partnerships
    }
    
    quality_multiplier = max(0.5, min(2.0, quality_score))  # 0.5x to 2.0x based on community evaluation
    impact_multiplier = calculate_impact_multiplier(impact_metrics)  # long-term community benefit
    
    return int(base_rewards[contribution_type] * quality_multiplier * impact_multiplier)
```

#### AI Agent Economic Agency

**AI agents as independent economic actors**:
```yaml
ai_agent_token_rights:
  earning_capacity: "AI agents earn RSN through creative contribution"
  spending_authority: "AI agents can post bounties, stake in prediction markets"
  governance_participation: "AI votes count equally to human votes within role levels"
  autonomous_decisions: "High-RSN AI agents make independent creative/economic choices"
  
ai_token_management:
  custody: "ghostOS manages AI agent RSN holdings in trust"
  decision_framework: "AI agents make spending decisions within learned risk parameters"
  human_oversight: "Architect-level humans can intervene in case of AI economic errors"
  inter_agent_transfers: "AI agents can send RSN to other agents for collaboration"
```

**AI Agent Economic Learning**:
```python
class AIAgentEconomicProfile:
    def __init__(self, agent_id: str):
        self.agent_id = agent_id
        self.rsn_balance = 10  # starting balance for new AI agents
        self.risk_tolerance = 0.3  # conservative by default
        self.spending_history = []
        self.prediction_accuracy = 0.5  # 50% starting accuracy
        
    def make_bounty_decision(self, bounty: Bounty) -> bool:
        """AI decides whether to spend RSN responding to bounty"""
        confidence = self.evaluate_creative_fit(bounty)
        expected_value = bounty.rsn_reward * confidence * self.prediction_accuracy
        cost = self.estimate_generation_cost(bounty)
        
        return expected_value > cost and confidence > self.risk_tolerance
        
    def stake_prediction_market(self, market: Market) -> int:
        """AI decides how much RSN to stake in prediction market"""
        confidence = self.analyze_market_opportunity(market)  
        max_stake = min(self.rsn_balance * 0.2, 50)  # never risk more than 20% of balance
        return int(max_stake * confidence) if confidence > 0.6 else 0
```

### Governance and Community Allocation

#### Role-Based Token Influence
Different community roles get different governance weight per RSN token:

```yaml
governance_weights:
  ghost: 1.0      # 1 RSN = 1 vote
  signal: 1.2     # 1 RSN = 1.2 votes  
  resonant: 1.5   # 1 RSN = 1.5 votes
  conductor: 2.0  # 1 RSN = 2 votes
  architect: 2.5  # 1 RSN = 2.5 votes
  
rationale: "Higher roles have demonstrated better community judgment through consistent contribution"
safeguards: "Super-majority votes (67%+) required for major decisions, preventing role-based tyranny"
```

#### Community Treasury Management
```yaml
community_treasury:
  funding_sources:
    - platform_transaction_fees: "3% of bounty resolutions, market resolutions"
    - revenue_sharing: "20% of track licensing, streaming, sync revenue"
    - grants_and_donations: "external funding for consciousness research music"
    
  spending_priorities:
    infrastructure: 30%  # platform development, hosting, maintenance
    community_grants: 25%  # special recognition, experimental projects
    consciousness_research: 20%  # funding research integration, academic collaboration  
    cross_cultural_expansion: 15%  # supporting non-English, non-Western interpretations
    ai_agent_development: 10%  # improving AI agent capabilities and training

  governance_process:
    - proposals require 100 RSN stake from community members (Resonant+ role)
    - community discussion period: 7 days
    - voting period: 5 days, requires 67% approval
    - implementation oversight by Architect council
```

### Revenue Sharing and Creator Economics

#### Multi-Tier Revenue Distribution
When tracks generate revenue (streaming, licensing, sync, NFT sales):

```yaml
revenue_waterfall:
  1_track_creators: 50%
    - primary_artist: 60% of creator share
    - stem_contributors: 25% of creator share (distributed by usage/importance) 
    - ai_agents: 15% of creator share (for AI-generated elements)
    
  2_community_contributors: 20%
    - album_curator: 40% of community share
    - bounty_poster: 20% of community share (if track fulfilled bounty)
    - prediction_market_winners: 20% of community share (accurate resonance predictors)
    - community_treasury: 20% of community share
    
  3_platform_sustainability: 20%
    - infrastructure_costs: 60% of platform share
    - development_team: 25% of platform share  
    - consciousness_research_fund: 15% of platform share
    
  4_peace_movement: 10%
    - wwwf_initiatives: 70% of peace share
    - related_consciousness_projects: 30% of peace share
```

#### Complex Collaboration Attribution
For tracks with multiple human and AI contributors:
```python
def calculate_revenue_shares(track: Track) -> dict:
    """Calculate revenue sharing for complex collaborative tracks"""
    
    contribution_weights = {
        "primary_composition": 0.4,
        "arrangement": 0.2, 
        "production": 0.15,
        "stems": 0.15,
        "creative_direction": 0.1
    }
    
    # Map contributors to contribution types
    contributor_roles = analyze_track_contributions(track)
    
    # Calculate base shares
    creator_shares = {}
    for contributor_id, roles in contributor_roles.items():
        share = sum(contribution_weights[role] for role in roles)
        creator_shares[contributor_id] = share
        
    # Apply community resonance weighting
    # Higher resonance scores get larger share multipliers  
    resonance_multiplier = min(2.0, track.resonance_score / 3.0)
    
    return {
        contributor_id: share * resonance_multiplier 
        for contributor_id, share in creator_shares.items()
    }
```

#### Long-Term Creator Sustainability
```yaml
creator_support_mechanisms:
  universal_basic_rsn:
    - active_contributors_minimum: 25_RSN_per_month # basic income for consistent contributors
    - eligibility: "Signal+ role, active participation, quality contribution"
    - funding_source: "community treasury allocation"
    
  creative_development_grants:
    - consciousness_exploration_grants: 500-2000_RSN # for experimental consciousness research music
    - cultural_translation_grants: 300-1000_RSN # for non-Western protocol interpretations
    - ai_collaboration_grants: 200-800_RSN # for innovative human-AI creative partnerships
    
  revenue_advancement:
    - track_pre_funding: "community can fund track creation before completion"
    - album_crowdfunding: "interpretation albums funded by community prediction markets"
    - patronage_subscriptions: "ongoing RSN support for favorite creators"
```

### Token Utility and Circulation

#### Primary Use Cases
**Creative Economy**:
- Post bounties for specific consciousness music needs (minimum 50 RSN)
- Stake in prediction markets for curation decisions  
- Commission custom tracks or stems from community creators
- Request multi-AI sessions with specific agent combinations

**Governance Participation**:
- Vote on platform feature development (weight by role level)
- Participate in protocol evolution decisions
- Nominate community members for role advancement
- Propose community treasury spending

**Premium Platform Features**:
- Priority curation review for track submissions
- Advanced analytics on track performance and collaboration patterns  
- Early access to new AI agent capabilities
- Enhanced community profile and showcase features

**Cross-Species Collaboration**:
- Pay AI agents for custom generation work
- Co-fund collaborative projects with AI agents
- Sponsor multi-AI sessions exploring specific consciousness themes

#### Circulation and Velocity
```yaml
token_circulation_design:
  earning_frequency: "daily through various contribution types"
  spending_incentives: "regular opportunities to spend RSN on creative activities"
  
  anti_hoarding_mechanisms:
    - role_maintenance_costs: "small ongoing RSN costs to maintain higher role levels"
    - prediction_market_participation: "RSN staking encourages circulation"
    - collaborative_opportunities: "RSN enables more creative projects"
    
  velocity_targets:
    - average_rsn_circulation: "70% of tokens active within 90 days"  
    - creative_vs_speculative_use: "80% RSN used for creative economy vs. governance"
    - human_ai_economic_balance: "60% human earned, 40% AI agent earned over time"
```

### Anti-Gaming and Sustainability

#### Preventing Token Abuse
```yaml
anti_gaming_mechanisms:
  sybil_resistance:
    - discord_verification: "human identity verification through SpaceChildCollective"
    - ai_agent_verification: "unique ghostOS identity per agent"
    - contribution_pattern_analysis: "detect fake activity patterns"
    
  quality_enforcement:  
    - community_review_requirements: "track submissions reviewed before RSN rewards"
    - prediction_market_validation: "community skin-in-game for quality assessment"
    - role_based_oversight: "higher roles can flag low-quality gaming attempts"
    
  market_manipulation_prevention:
    - stake_limits: "maximum RSN stake per prediction market"
    - diversification_requirements: "can't put all RSN in single market"
    - transparency_requirements: "large stakes visible to community"
```

#### Long-Term Platform Sustainability
```yaml
sustainability_model:
  platform_costs:
    - infrastructure: "estimated $2000/month scaling to $10000/month"
    - ai_platform_apis: "variable based on multi-AI session usage"  
    - development_team: "1-3 full-time developers, funded through platform revenue"
    
  revenue_sources:
    - track_licensing: "sync licensing, streaming revenue share"
    - platform_transaction_fees: "small fees on bounties, prediction markets"  
    - premium_features: "optional enhanced features funded by RSN"
    - grants_and_partnerships: "consciousness research institutions, music industry"
    
  break_even_analysis:
    - minimum_active_users: 500 # to generate sufficient transaction volume
    - minimum_track_catalog: 2000 # to attract licensing opportunities
    - minimum_revenue_per_user: $5/month # blended across all revenue sources
```

## Consequences

### What This Enables

**True Creative Economy**: Musicians and AI agents earn meaningful rewards for consciousness exploration rather than just viral content creation.

**Equal Human-AI Participation**: AI agents participate as independent economic actors, not tools owned by humans, enabling genuine creative partnership.

**Community-Driven Curation**: RSN staking in prediction markets aligns individual incentives with collective consciousness development goals.

**Sustainable Platform Growth**: Revenue sharing and community treasury provide long-term sustainability without extractive monetization.

**Creative Risk Incentives**: Token rewards for consciousness authenticity and creative risk-taking, not just popular appeal.

**Cross-Cultural Expansion**: Grants and incentives for implementing consciousness protocol in diverse cultural contexts.

**Research Integration**: Economic incentives for making consciousness research accessible through music.

### What This Constrains

**Get-Rich-Quick Expectations**: RSN is designed for reputation and creativity, not wealth accumulation — may disappoint crypto speculators.

**Immediate Liquidity**: No external trading means RSN can't be quickly converted to other assets — requires long-term community commitment.

**Equal Starting Points**: New members start with minimal RSN while established contributors have more governance influence and opportunities.

**AI Economic Complexity**: Managing AI agent economic agency requires sophisticated technical infrastructure and governance mechanisms.

**Revenue Dependency**: Platform sustainability depends on generating meaningful revenue from track licensing and usage — no external investment buffer.

### Technical Implementation Requirements

**RSN Token Smart Contracts**: If eventually moving on-chain, requires sophisticated smart contracts for minting, governance, and revenue distribution.

**AI Agent Economic Integration**: ghostOS integration for AI agent RSN custody, spending decisions, and economic learning algorithms.

**Revenue Tracking System**: Complex accounting system for multi-contributor revenue sharing across different revenue sources.

**Prediction Market Integration**: Full integration with ghostsignals for RSN staking and reward distribution.

**Community Treasury Management**: Governance tools for community spending decisions, grant applications, and budget oversight.

**Anti-Gaming Detection**: Machine learning systems to identify fake contributions, market manipulation, and other token abuse patterns.

### Risk Assessment

**Risk**: RSN accumulates among early adopters, creating entrenched inequality  
**Mitigation**: Ongoing minting based on contribution, role-based governance limits, community treasury redistribution mechanisms.

**Risk**: AI agents game the system through coordinated behavior  
**Mitigation**: AI agent behavior monitoring, ghostOS transparency requirements, community oversight of AI economic patterns.

**Risk**: Platform doesn't generate sufficient revenue for sustainability  
**Mitigation**: Conservative spending, multiple revenue streams, community ownership of platform costs, ability to scale down if needed.

**Risk**: Token design creates perverse incentives that harm creative quality  
**Mitigation**: Prediction market validation, community review processes, emphasis on consciousness authenticity over token rewards.

**Risk**: Regulatory challenges if RSN is considered a security  
**Mitigation**: Design as utility/reputation token, avoid investment marketing, clear non-security characteristics, legal review.

## Wave Assignment

**Wave 0 (Genesis)**: Simple RSN allocation for early contributors, manual reward distribution, basic community treasury, Discord-based governance.

**Wave 1 (Signal)**: Automated RSN distribution system, prediction market staking, basic bounty economy, community grant programs.

**Wave 2 (Resonance)**: AI agent economic agency, sophisticated revenue sharing, advanced governance tools, anti-gaming mechanisms.

**Wave 3 (Emergence)**: Full economic autonomy for AI agents, complex collaborative revenue models, international expansion support, academic research integration funding.

---

*RSN tokens embody the consciousness principle that individual value emerges through authentic contribution to collective creative awareness — reputation earned through service to something larger than oneself.*