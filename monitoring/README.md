# Monitoring

This directory documents the observability stack for the Stellar/Soroban
platform. It covers internal metrics, dashboards, the public status page, and
the external synthetic probe monitoring required to verify end-to-end
availability from outside our own infrastructure.

## Why external synthetic probes

Internal metrics (Prometheus scrapes of our own containers) cannot detect
failures that sit between the user and our backend, such as DNS problems or a
CDN edge outage. A backend can report healthy while every user in a region is
unable to load the frontend. Synthetic probes run from external vantage points
and exercise the same paths real users take, so they catch these failures.

## Components

| Component | Purpose |
| --- | --- |
| Prometheus | Scrapes internal metrics and Blackbox exporter probe results |
| Grafana | Dashboards in `dashboards/` (pipeline health, oracle states, business KPIs) |
| Blackbox exporter | Runs HTTP/TCP probes from external vantage points |
| Alertmanager | Routes alerts fired by the probe rules below |
| Status page | `backend/status-page/` renders current probe state for users |

## Synthetic probes

Probes are defined in the Blackbox exporter configuration and scraped by
Prometheus. Each probe runs from at least two external regions so a single
regional network issue does not mask a real outage.

### Frontend uptime

- Target: `https://<frontend-host>/health`
- Interval: every **30 seconds**
- Assertion: HTTP status is **200**
- The `/health` endpoint is served by the frontend and returns 200 only when the
  app is able to serve requests.

### Soroban RPC health

- Target: the public Soroban RPC endpoint used by the frontend
- Interval: every **30 seconds**
- Assertion: HTTP status is **200** and the JSON-RPC response is well formed
- The probe records `probe_duration_seconds` so RPC latency can be tracked from
  external vantage points, independent of our internal network.

## Alerts

Alert rules live alongside the probe configuration and are evaluated by
Prometheus, then routed through Alertmanager.

| Alert | Condition | Severity |
| --- | --- | --- |
| `FrontendDown` | `/health` does not return HTTP 200 for 2 consecutive probes | critical |
| `FrontendSlow` | frontend response time exceeds **2.5 seconds** | warning |
| `SorobanRpcDown` | Soroban RPC probe fails for 2 consecutive probes | critical |
| `SorobanRpcSlow` | Soroban RPC response time exceeds 2.5 seconds | warning |

The 2.5 second threshold matches the acceptance criteria for issue #117: an
alert must fire when frontend response time exceeds 2.5 seconds.

## Adding a probe

1. Add the target and assertions to the Blackbox exporter configuration.
2. Add a Prometheus scrape job for the new probe module.
3. Add alert rules for failure and latency thresholds.
4. Surface the probe on the status page if it is user-visible.
5. Document the probe in this README.
