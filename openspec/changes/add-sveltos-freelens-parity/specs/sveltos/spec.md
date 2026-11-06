## MODIFIED Requirements

### Requirement: Cluster overview

Each cluster row SHALL show type, namespace, name, reported readiness, reported connectivity where available, paused/deleting state, and deployment counts grouped by provisioning state. The dashboard MUST NOT treat CAPI control-plane initialization as proof of downstream connectivity. Missing readiness or connectivity evidence SHALL read unknown or not reported rather than healthy.

#### Scenario: Initialized CAPI cluster

- **WHEN** CAPI reports its control plane initialized but supplies no connectivity evidence
- **THEN** the row shows the initialization/readiness evidence separately and connectivity as not reported
