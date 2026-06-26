## ADDED Requirements

### Requirement: Project Map generation preserves workspace ownership
The system MUST bind each Project Map generation run to the workspace, storage key, and storage location that were active when the run started, and MUST NOT let later workspace or storage-view switches redirect that run's dataset updates, persistence writes, or UI state into another workspace or storage view.

#### Scenario: In-flight run completes after workspace switch
- **WHEN** a Project Map generation run starts for workspace A
- **AND** the user switches to workspace B before the run emits progress, completion, or failure
- **THEN** the run SHALL continue using workspace A's storage key and worker-local dataset for any persisted run update
- **AND** workspace B's Project Map dataset and UI state SHALL NOT receive nodes, sources, relationships, or run metadata from workspace A

#### Scenario: In-flight run completes after storage view switch
- **WHEN** a Project Map generation run starts for the global storage view of workspace A
- **AND** the user switches to the project storage view of the same workspace before the run emits progress, completion, or failure
- **THEN** the run SHALL continue writing only to the global storage location it started with
- **AND** the project storage view's UI state SHALL NOT receive nodes, sources, relationships, or run metadata from the global run

#### Scenario: Worker write requires matching manifest storage key
- **WHEN** a Project Map worker attempts to persist a dataset for a workspace
- **THEN** the dataset manifest `storageKey` MUST match the storage key derived for that target workspace
- **AND** a mismatch MUST reject the write instead of rewriting ownership or silently falling back to the active workspace

### Requirement: Project Map storage rejects ownership mismatches
The Project Map storage boundary MUST treat persisted snapshot ownership as a contract and MUST reject reads or writes whose manifest storage key does not match the requested workspace storage key.

#### Scenario: Backend rejects mismatched manifest on write
- **WHEN** the frontend calls the Project Map snapshot write command for workspace A
- **AND** the incoming files include a `manifest.json` whose `storageKey` belongs to workspace B
- **THEN** the backend MUST reject the write with an ownership mismatch error
- **AND** the backend MUST NOT write any snapshot files into workspace A's Project Map directory

#### Scenario: Frontend quarantines mismatched persisted snapshot on read
- **WHEN** the Project Map read path loads files for workspace A
- **AND** the persisted `manifest.json` has a `storageKey` that does not match workspace A's expected storage key
- **THEN** the frontend MUST NOT render that persisted snapshot as a valid Project Map dataset
- **AND** the user-visible dataset SHALL fall back to an empty or error/quarantined state for workspace A
