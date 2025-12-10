# Why YAML for Financial Models?

Forge uses YAML because it's the optimal format for AI-native financial modeling.

## LLMs Are Trained on YAML

Modern code LLMs are heavily trained on YAML:

| Dataset | YAML Files | Source |
|---------|-----------|--------|
| [The Stack](https://huggingface.co/datasets/bigcode/the-stack) | 13.4 million | GitHub (358 languages) |
| [K8s YAML Dataset](https://huggingface.co/datasets/substratusai/the-stack-yaml-k8s) | 276,520 | Kubernetes manifests |
| [The Stack v2](https://huggingface.co/datasets/bigcode/the-stack-v2) | 4x larger | 619 languages |

YAML is ubiquitous in:
- **Kubernetes** — every K8s manifest is YAML
- **CI/CD** — GitHub Actions, GitLab CI, CircleCI
- **Infrastructure** — Docker Compose, Ansible, Terraform
- **Configuration** — virtually every modern tool

**Result**: LLMs understand YAML syntax deeply. They can read, write, and reason about YAML with high accuracy.

## YAML vs CSV for Financial Models

| Capability | YAML | CSV |
|------------|------|-----|
| **Formulas** | ✓ `"=revenue - costs"` | ✗ Just flat data |
| **Rich metadata** | ✓ units, notes, sources | ✗ No structure |
| **Nested structures** | ✓ Tables, scenarios, configs | ✗ Flat rows only |
| **Git-diffable** | ✓ Meaningful diffs | ✓ But noisy |
| **Human-readable** | ✓ Indentation-based | ✓ But no context |
| **LLM training data** | ✓ 13M+ files | ✓ Common but flat |
| **Token efficiency** | Moderate | Best for flat data |

### The Formula Problem

CSV cannot represent formulas:

```csv
revenue,costs,profit
1000000,400000,600000
```

Where's the logic? Lost. The AI sees numbers, not relationships.

YAML preserves the logic:

```yaml
revenue: 1000000
costs: 400000
profit: "=revenue - costs"
```

The AI sees the formula. It can reason about it, modify it, validate it.

### The Metadata Problem

CSV cannot hold metadata:

```csv
revenue
1000000
```

What unit? What source? What assumptions? Unknown.

YAML (v5.0.0 schema) captures everything:

```yaml
revenue:
  value: 1000000
  unit: USD
  notes: "FY2025 projected revenue"
  source: "Sales forecast Q3"
```

## Token Efficiency Trade-offs

| Format | Tokens (flat data) | Tokens (nested) | LLM Accuracy |
|--------|-------------------|-----------------|--------------|
| CSV | Lowest | N/A | 44.3% |
| YAML | Moderate | Good | Higher |
| JSON | High (+40%) | Verbose | Good |
| Markdown-KV | Low | Limited | 60.7% |

**Source**: [Best Input Format for LLMs](https://www.improvingagents.com/blog/best-input-data-format-for-llms)

CSV wins on raw token count for flat tables. But financial models aren't flat:
- Formulas reference other cells
- Metadata adds context
- Scenarios create hierarchies
- Tables have computed columns

For this structure, YAML is optimal.

## Excel MCP Overhead

Using Excel with AI requires tools:

| Approach | Token Overhead |
|----------|---------------|
| MCP Excel Server | ~12,000 tokens (tool definitions) |
| Convert to CSV | Loses formulas, metadata |
| Convert to JSON | +40% token bloat |
| **Native YAML** | Zero overhead |

**Source**: [Anthropic MCP Engineering](https://www.anthropic.com/engineering/code-execution-with-mcp)

## Summary

YAML is ideal for AI-native financial modeling because:

1. **LLMs know YAML** — 13M+ training files
2. **Formulas preserved** — AI sees logic, not just numbers
3. **Metadata captured** — units, notes, sources
4. **Git-native** — meaningful version control
5. **Zero tool overhead** — no MCP, no conversion

CSV is for data dumps. YAML is for models.

---

## References

- [The Stack Dataset](https://huggingface.co/datasets/bigcode/the-stack) — 3TB of code, 13.4M YAML files
- [The Stack v2](https://huggingface.co/datasets/bigcode/the-stack-v2) — 4x larger, 619 languages
- [K8s YAML Dataset](https://www.substratus.ai/blog/k8s-yaml-dataset) — 276k Kubernetes manifests for LLM training
- [Token Format Comparison](https://medium.com/@rajeev.bit30/tokenization-comparison-token-usage-across-csv-json-yaml-and-toon-for-llm-interactions-3a2df3956587) — CSV vs JSON vs YAML tokens
- [Best Format for LLMs](https://www.improvingagents.com/blog/best-input-data-format-for-llms) — 11 formats tested
- [MCP Context Overhead](https://www.anthropic.com/engineering/code-execution-with-mcp) — Tool definitions cost ~12k tokens
- [TOON Format](https://www.infoq.com/news/2025/11/toon-reduce-llm-cost-tokens/) — 40% fewer tokens than JSON
