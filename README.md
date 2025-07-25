# `rust-txpool`

> ![WARNING]
> Pure Generated Code, there be slooop

## Metrics Collected

### Type Wrapper Metrics
- **`txpool.type_wrapper.instances`**: Count of each type wrapper found during parsing
  - Labels: `wrapper_type` (e.g., "TxpoolContent", "Transaction", "Eip1559")
  - Shows the distribution of different transaction types and structures

### Performance Metrics
- **`txpool.input.bytes`**: Size of input data in bytes
- **`txpool.output.bytes`**: Size of output JSON in bytes
- **`txpool.parse.duration_ms`**: Total parse time in milliseconds
- **`txpool.content.parse_duration_ms`**: Time spent parsing txpool content specifically
- **`txpool.field.replacements`**: Number of field name quotations performed

### Error Metrics
- **`txpool.parse.errors`**: Count of parsing errors
  - Labels: `error_type`, `error_line`, `error_column`

## Setup

### Environment Variables
- `OTLP_ENDPOINT`: The OTLP gRPC endpoint (default: `http://localhost:4317`)

### Running with OpenTelemetry Collector

1. Start an OpenTelemetry Collector:
```yaml
# otel-collector-config.yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317

exporters:
  prometheus:
    endpoint: "0.0.0.0:8889"
  logging:
    loglevel: debug

service:
  pipelines:
    metrics:
      receivers: [otlp]
      exporters: [prometheus, logging]
    logs:
      receivers: [otlp]
      exporters: [logging]
    traces:
      receivers: [otlp]
      exporters: [logging]
```

2. Run the collector:
```bash
otelcol --config otel-collector-config.yaml
```

3. Run the parser:
```bash
cast tx-pool content | ./target/release/rust-txpool
```

4. View metrics at `http://localhost:8889/metrics`

## Example Output

```
21:14:52.217 metric rust_txpool count of txpool.type_wrapper.instances is 3
21:14:52.217 debug rust_txpool Found 3 instances of type wrapper: Transaction
21:14:52.218 metric rust_txpool count of txpool.field.replacements is 156
21:14:52.218 metric rust_txpool last of txpool.content.parse_duration_ms is 2
21:14:52.218 info rust_txpool Successfully parsed txpool content in 2ms
```

## Grafana Dashboard

You can create a Grafana dashboard with these queries:

1. **Type Wrapper Distribution**:
   ```promql
   sum by (wrapper_type) (txpool_type_wrapper_instances)
   ```

2. **Parse Performance**:
   ```promql
   rate(txpool_parse_duration_ms[5m])
   ```

3. **Data Volume**:
   ```promql
   rate(txpool_input_bytes[5m])
   rate(txpool_output_bytes[5m])
   ```

4. **Error Rate**:
   ```promql
   rate(txpool_parse_errors[5m])
   ```

