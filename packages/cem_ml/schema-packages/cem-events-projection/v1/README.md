# CEM Events Projection Schema

This package defines the semantic CEM event-stream projection layer:

- schema URL: `https://cem.dev/ns/projection/events/1`
- primary content type: `application/vnd.cem.events+cem-bin`
- debug/interchange view: `application/vnd.cem.events+json`

The events projection is designed for replay, multicast, and incremental
consumers. JSON output remains a view over this event layer.
