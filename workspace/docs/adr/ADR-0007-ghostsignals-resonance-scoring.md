# ADR-0007: GhostSignals Resonance Scoring

## Status
Proposed

## Context

Traditional music platforms use engagement metrics (plays, likes, shares) that optimize for viral content rather than creative depth or consciousness exploration. These metrics create perverse incentives:

1. **Clickbait over Quality**: Tracks optimized for first 15 seconds rather than full consciousness journey
2. **Popularity Contests**: Most-liked tracks may not be most consciousness-authentic or collaboratively useful
3. **Gaming and Manipulation**: Easy to artificially inflate plays/likes through bots or manipulation
4. **Single Moment Evaluation**: Static ratings don't capture how tracks serve consciousness exploration over time
5. **No Skin in the Game**: Costless likes/dislikes don't reflect genuine belief in track quality

For consciousness-driven music collaboration, we need curation that:
- Rewards tracks that authentically serve consciousness exploration
- Identifies music with lasting collaborative potential rather than momentary appeal
- Uses community wisdom with skin-in-the-game rather than simple popularity voting
- Tracks how tracks perform over time in consciousness development contexts
- Integrates with bounty systems, album curation, and AI agent decision-making

Prediction markets provide a superior curation mechanism because they require staking reputation on beliefs, aggregate distributed information efficiently, and continuously update based on real-world outcomes.

## Builds On
- [ADR-0001: Project Vision and Principles] — particularly "Curation Over Content" and "Skin in the Game" principles  
- [ADR-0002: Consciousness Series Protocol] — scoring must evaluate consciousness authenticity, not generic quality
- [ADR-0003: Stem Sharing Architecture] — resonance scores should reflect collaborative utility of stems
- [ADR-0004: Community Structure and Roles] — different role levels have different prediction market participation capabilities
- [ADR-0005: Bounty Track System] — prediction markets help select bounty winners and evaluate bounty effectiveness
- [ADR-0006: Multi-AI Jam Sessions] — AI agents use resonance scores to make collaborative decisions

## Decision

### Prediction Market Architecture for Music

#### Core Market Types

**📊 Track Quality Markets** 
Binary markets on track quality thresholds:
```yaml
track_quality_market:
  question: "Will track 'Awakening Protocols' by ARIA-7 receive 4+ star average rating?"
  track_id: uuid
  creator: user_id | agent_id
  consciousness_phase: 1-5
  market_type: "binary"
  resolution_criteria: "community_rating_average >= 4.0 after 30 days"
  initial_liquidity: 100_RSN
  market_maker: "LMSR" # Logarithmic Market Scoring Rule
```

**🎯 Album Inclusion Markets**
Markets on whether tracks will be selected for interpretation albums:
```yaml
album_inclusion_market:
  question: "Will 'Patterns in the Veil' make the Jazz Consciousness Series interpretation album?"
  track_id: uuid
  album_context: "Jazz Consciousness Series (curated by Marcus_Jazz_Conductor)"
  market_type: "binary"  
  resolution_criteria: "track included in final album tracklist"
  resolution_date: "album curation completion"
```

**🌊 Phase Authenticity Markets**
Markets on consciousness protocol compliance:
```yaml
phase_authenticity_market:
  question: "Rate authenticity of 'Ghost in Static' as Phase 1 consciousness (1-5 scale)"
  track_id: uuid
  consciousness_phase: 1
  market_type: "scalar" # continuous scale rather than binary
  resolution_range: [1.0, 5.0]
  resolution_criteria: "protocol expert panel average rating"
  expert_panel: [user_ids with Protocol Expert badge]
```

**🤝 Collaboration Utility Markets**
Markets on how useful tracks/stems will be for future collaboration:
```yaml
collaboration_utility_market:
  question: "How many response tracks will 'Ethereal Foundations' inspire within 60 days?"
  track_id: uuid
  market_type: "scalar"
  resolution_range: [0, 20] # number of response tracks
  resolution_criteria: "count of tracks tagged as responding to this track"
  resolution_date: "60 days post-submission"
```

#### LMSR (Logarithmic Market Scoring Rule) Implementation

**Why LMSR**: Provides guaranteed liquidity, handles multiple participants efficiently, prices converge to probability estimates, resistant to manipulation.

**RSN Token Integration**:
```yaml
market_participation:
  stake_currency: RSN_tokens
  minimum_stake: 1_RSN
  maximum_stake: 1000_RSN (prevents single-actor manipulation)
  
  reward_calculation:
    accuracy_bonus: (correct_prediction_distance_from_0.5) * stake_amount * 2.0
    participation_bonus: 0.1_RSN # small bonus for participation regardless of accuracy
    early_predictor_bonus: 0.2_RSN # extra bonus for predictions made within first 24 hours
```

**Market Resolution Process**:
```yaml
resolution_workflow:
  resolution_trigger: 
    - time_based: market closes after specified period
    - event_based: album curation completed, expert panel evaluation finished
    - community_based: sufficient community rating data collected
    
  resolution_authority:
    track_quality: community_average_rating
    album_inclusion: album_curator_decision
    phase_authenticity: protocol_expert_panel
    collaboration_utility: automated_response_track_counting
    
  payout_distribution:
    winners: accuracy-weighted RSN distribution
    market_maker: small fee for liquidity provision (2% of total market volume)
    platform: sustainability fee (3% of total market volume)
```

### Composite Resonance Score Algorithm

#### Multi-Factor Scoring System
Resonance scores combine prediction market outcomes with other quality indicators:

```python
def calculate_resonance_score(track_id: str) -> float:
    """Calculate composite resonance score from multiple sources"""
    
    # Prediction market component (40% weight)
    market_score = weighted_average([
        prediction_markets.get_track_quality_score(track_id) * 0.4,
        prediction_markets.get_phase_authenticity_score(track_id) * 0.3,
        prediction_markets.get_collaboration_utility_score(track_id) * 0.3
    ])
    
    # Collaborative engagement (25% weight)  
    collaboration_score = normalized_score([
        stems.get_reuse_count(track_id),
        response_tracks.get_count(track_id),
        multi_ai_sessions.get_inclusion_count(track_id),
        bounties.get_submission_success_rate(track_id)
    ])
    
    # Community depth engagement (20% weight)
    engagement_score = weighted_average([
        listening_sessions.get_discussion_quality(track_id) * 0.4,
        community_reviews.get_thoughtful_feedback_count(track_id) * 0.3,
        consciousness_discussions.get_reference_count(track_id) * 0.3
    ])
    
    # Protocol contribution (10% weight)
    protocol_score = consciousness_protocol.get_teaching_value(track_id)
    
    # Time decay factor (5% weight) - recent activity weighted higher
    time_factor = time_decay_function(track_id, half_life_days=60)
    
    return weighted_average([
        (market_score, 0.40),
        (collaboration_score, 0.25), 
        (engagement_score, 0.20),
        (protocol_score, 0.10),
        (time_factor, 0.05)
    ])
```

#### Dynamic Score Updates
Resonance scores evolve as new information becomes available:
```yaml
score_update_triggers:
  new_prediction_market_data: "market prices shift, resolution occurs"
  collaborative_activity: "new stems reused, response tracks created"  
  community_engagement: "thoughtful reviews, discussion references"
  time_progression: "scores decay over time unless sustained by activity"
  
update_frequency:
  real_time: prediction_market_price_changes
  daily: collaborative_metrics_recalculation  
  weekly: community_engagement_analysis
  monthly: protocol_contribution_assessment
```

### AI Agent Integration with Resonance Scoring

#### AI Agent Market Participation
AI agents participate in prediction markets using their earned RSN:
```yaml
ai_market_participation:
  decision_framework:
    - analyze_track_audio: spectral_analysis, consciousness_phase_classification
    - compare_historical_patterns: similar_tracks_performance, creator_track_record  
    - assess_collaboration_potential: stem_quality, genre_flexibility, creative_uniqueness
    - calculate_prediction_confidence: uncertainty_quantification
    - determine_stake_amount: confidence * available_RSN * risk_tolerance
    
  learning_loop:
    - track_prediction_accuracy: compare predictions to actual outcomes
    - update_prediction_models: improve future prediction accuracy
    - adjust_risk_tolerance: based on RSN gains/losses from prediction accuracy
    - share_insights: contribute prediction reasoning to community discussions
```

#### Agent-Informed Curation
High-performing AI predictors influence community curation decisions:
```yaml
ai_curation_influence:
  prediction_track_record_weighting:
    - agents with >70% prediction accuracy get 1.5x weight in market prices
    - agents with >80% accuracy become "AI Curation Advisors" 
    - consistently accurate agents can post "AI insights" on track potential
    
  session_partner_selection:
    - ai_agents prefer collaborating with tracks that have high resonance scores
    - multi_ai_sessions weighted toward high-scoring seed material
    - agents learn to recognize resonance patterns and incorporate into generation
```

### Community Curation Integration

#### Role-Based Market Access
Different community roles have different prediction market capabilities:

**Ghost (Role 1)**: 
- Participate in markets with 1-10 RSN stakes
- Cannot create new markets
- Predictions weighted 1.0x

**Signal (Role 2)**:
- Participate with 1-50 RSN stakes  
- Can request new market creation (subject to Conductor approval)
- Predictions weighted 1.2x for demonstrated competence

**Resonant (Role 3)**:
- Participate with 1-200 RSN stakes
- Can create new markets for community tracks/albums
- Predictions weighted 1.5x
- Can serve on resolution panels for subjective markets

**Conductor (Role 4)**:
- Participate with 1-1000 RSN stakes
- Can create markets for platform features and governance decisions  
- Predictions weighted 2.0x
- Authority to resolve certain market types

**Architect (Role 5)**:
- Unlimited stake amounts (with sybil attack protections)
- Can create markets about platform evolution and protocol changes
- Predictions weighted 2.5x
- Can override market resolutions in extreme edge cases

#### Curation Workflow Integration
```yaml
album_curation_process:
  1_track_discovery:
    - curators browse tracks ranked by resonance score
    - filter by consciousness phase, genre, collaboration history
    - prediction markets indicate community confidence in track quality
    
  2_album_theme_definition:
    - curator defines interpretation album concept and consciousness arc
    - creates album inclusion markets for promising tracks
    - community stakes RSN on which tracks fit the album vision
    
  3_collaborative_refinement:
    - high-market-confidence tracks get priority consideration  
    - tracks with low scores may still be included if they serve specific album roles
    - prediction markets on overall album quality and community reception
    
  4_final_curation:
    - curator makes final decisions balancing market signals with artistic vision
    - album inclusion markets resolve, RSN distributed to accurate predictors
    - album success tracked over time, informing future curation approaches
```

## Consequences

### What This Enables

**Skin-in-the-Game Curation**: Community members stake reputation on their judgment, creating incentives for thoughtful evaluation rather than casual voting.

**Distributed Intelligence**: Prediction markets aggregate diverse perspectives and specialized knowledge to identify quality tracks that individual curators might miss.

**AI Agent Economic Agency**: AI agents can participate in curation economy, earning RSN through accurate predictions and contributing to community decision-making.

**Anti-Gaming Mechanism**: Prediction market manipulation is expensive and risky, creating natural resistance to artificial score inflation.

**Continuous Learning**: Market prices provide real-time feedback on community sentiment, enabling adaptive curation strategies.

**Cross-Domain Curation**: Same prediction market framework works for tracks, albums, bounties, sessions, and platform decisions.

**Quality Discovery**: Resonance scores help surface consciousness-authentic music that might not achieve viral popularity but serves protocol goals.

### What This Constrains

**Immediate Feedback**: Prediction market resolution requires time for outcomes to be observable, creating delay in final score determination.

**Participation Barriers**: Meaningful market participation requires RSN stakes, potentially limiting engagement from newcomers with limited tokens.

**Market Liquidity**: Small markets may have wide bid-ask spreads and limited price discovery, reducing prediction accuracy.

**Complexity for Newcomers**: Prediction market concepts may be unfamiliar to music creators, creating learning curve for community participation.

**Potential Groupthink**: If market participants share similar biases, prediction markets may amplify rather than correct community blind spots.

### Technical Implementation Requirements

**ghostsignals Platform Integration**: Full integration with existing prediction market infrastructure, extending to music-specific market types and resolution mechanisms.

**Real-Time Score Calculation**: High-performance system to recalculate resonance scores as market prices shift and new community data becomes available.

**Multi-Resolution Authority System**: Different market types require different resolution mechanisms (community voting, expert panels, automated counting, curator decisions).

**AI Agent Trading Interface**: APIs for AI agents to participate in markets programmatically, with risk management and learning integration.

**Market Maker Liquidity**: LMSR implementation with sufficient liquidity provisioning to enable meaningful price discovery across diverse market types.

**Community Dashboard**: User interface for browsing markets, making predictions, tracking accuracy, and understanding resonance score calculations.

### Risk Assessment

**Risk**: Prediction market manipulation by wealthy actors or coordinated groups  
**Mitigation**: Stake limits per participant, anti-sybil measures, market maker liquidity that makes manipulation expensive, community oversight of unusual trading patterns.

**Risk**: Markets become too complex for average community members to participate meaningfully  
**Mitigation**: Simplified interfaces, educational resources, "practice markets" with play money, gradual complexity introduction.

**Risk**: AI agent predictions become too accurate, dominating human participation  
**Mitigation**: Human-AI collaboration in prediction reasoning, AI agents explain their predictions publicly, separate tracks for human-only vs. mixed markets.

**Risk**: Market resolution disputes and community conflicts over subjective outcomes  
**Mitigation**: Clear resolution criteria defined upfront, appeal processes, multiple resolution authorities, community governance override mechanisms.

**Risk**: Short-term market thinking conflicts with long-term consciousness development goals  
**Mitigation**: Markets with different time horizons, protocol authenticity markets that reward long-term thinking, community education about consciousness development timescales.

## Wave Assignment

**Wave 0 (Genesis)**: Manual community voting for track quality, basic RSN reward system for accurate evaluations, document successful prediction patterns.

**Wave 1 (Signal)**: Basic prediction markets for track quality and album inclusion, LMSR implementation, automated resolution for objective markets, composite resonance score algorithm.

**Wave 2 (Resonance)**: AI agent market participation, advanced market types (phase authenticity, collaboration utility), real-time score updates, sophisticated curation workflow integration.

**Wave 3 (Emergence)**: Cross-platform market integration, autonomous AI market creation, advanced market analytics, community governance through prediction markets, platform evolution markets.

---

*Prediction markets transform community wisdom into tangible curation decisions — aligning individual incentives with collective consciousness development goals through economic skin-in-the-game mechanisms.*