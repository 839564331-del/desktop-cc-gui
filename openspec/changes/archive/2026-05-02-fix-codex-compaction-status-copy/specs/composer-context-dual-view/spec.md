## MODIFIED Requirements

### Requirement: Shared State Source Consistency
Both views MUST read context usage and compaction signals from the same conversation state source.

#### Scenario: Token usage consistency across two views
- **WHEN** `thread/tokenUsage/updated` updates `latestTokenUsageInfo`
- **THEN** legacy view and new view SHALL reflect the same token usage snapshot
- **AND** neither view SHALL introduce an alternative token calculation source

#### Scenario: Compaction status consistency across two views
- **WHEN** conversation enters compacting or freshly completed compaction states
- **THEN** both views SHALL present state-consistent messaging
- **AND** state transitions SHALL be derived from explicit thread lifecycle state rather than preserved historical compaction messages alone

### Requirement: Tooltip Detail Format For Codex Summary
The codex summary tooltip MUST present textual details without an extra progress bar, and MUST distinguish lifecycle status from usage snapshot freshness.

#### Scenario: Tooltip displays required details
- **WHEN** user hovers the codex context summary indicator
- **THEN** tooltip SHALL show total token consumption
- **AND** tooltip SHALL show context usage ratio as percent and used/window
- **AND** tooltip SHALL show compaction status text when available

#### Scenario: Completed compaction with stale usage snapshot stays truthful
- **WHEN** the current Codex thread has just completed compaction
- **AND** the latest background-information usage snapshot has not yet refreshed to the post-compaction value
- **THEN** tooltip SHALL keep showing the latest available usage snapshot
- **AND** tooltip SHALL present an explicit sync-pending completion hint instead of implying that compaction failed or never happened

#### Scenario: Historical compaction messages do not pin current tooltip state
- **WHEN** thread history restore includes preserved compaction messages from earlier lifecycles
- **AND** the current thread is no longer compacting or freshly completing compaction
- **THEN** tooltip SHALL return to neutral current-state messaging
- **AND** the system SHALL NOT mark the current tooltip state as completed solely because historical compaction messages still exist

#### Scenario: No redundant progress bar in tooltip
- **WHEN** tooltip is rendered for codex summary
- **THEN** tooltip SHALL NOT render an additional bar-style progress indicator
