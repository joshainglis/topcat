# Skill Examples

## Example 1: Simple Tool Skill

### SKILL.md
```markdown
---
name: formatting-json
description: Formats, validates, and transforms JSON data including prettification, minification, and schema validation. Use when working with JSON files, API responses, or configuration files.
---

# Formatting JSON

## Quick Commands

\`\`\`bash
# Prettify
jq '.' input.json > pretty.json

# Minify
jq -c '.' input.json > minified.json

# Validate
jq empty input.json && echo "Valid JSON"
\`\`\`

## Common Transformations

### Extract Field
\`\`\`bash
jq '.data.items[]' input.json
\`\`\`

### Filter Objects
\`\`\`bash
jq '.users[] | select(.age > 21)' users.json
\`\`\`

For complex transformations, see [reference/transformations.md](reference/transformations.md)
```

## Example 2: Workflow Skill

### SKILL.md
```markdown
---
name: deploying-containers
description: Guides Docker container deployment including building, testing, and pushing to registries. Use when containerizing applications, setting up CI/CD, or deploying to production.
---

# Deploying Containers

## Deployment Workflow

Copy this checklist:

\`\`\`
Deployment Progress:
- [ ] Build image: docker build -t app:latest .
- [ ] Test locally: docker run --rm app:latest test
- [ ] Tag for registry: docker tag app:latest registry/app:v1.0
- [ ] Push to registry: docker push registry/app:v1.0
- [ ] Deploy to environment: kubectl apply -f deployment.yaml
- [ ] Verify deployment: kubectl rollout status deployment/app
\`\`\`

## Quick Reference

| Action | Command |
|--------|---------|
| Build | \`docker build -t name .\` |
| Run | \`docker run -p 8080:8080 name\` |
| Push | \`docker push registry/name\` |

For troubleshooting, see [reference/debugging.md](reference/debugging.md)
```

## Example 3: Analysis Skill

### SKILL.md
```markdown
---
name: analyzing-performance
description: Analyzes application performance using profiling tools and metrics. Use when investigating slowdowns, optimizing code, or establishing performance baselines.
---

# Analyzing Performance

## Quick Analysis

\`\`\`bash
# CPU profile (30 seconds)
perf record -g -p $(pgrep appname) sleep 30
perf report

# Memory snapshot
pmap -x $(pgrep appname)
\`\`\`

## Profiling Workflow

\`\`\`
Analysis Checklist:
- [ ] Establish baseline metrics
- [ ] Identify bottlenecks
- [ ] Profile specific operations
- [ ] Analyze results
- [ ] Implement optimizations
- [ ] Verify improvements
\`\`\`

For detailed profiling guides:
- **CPU Profiling**: See [reference/cpu-profiling.md](reference/cpu-profiling.md)
- **Memory Analysis**: See [reference/memory-analysis.md](reference/memory-analysis.md)
- **I/O Analysis**: See [reference/io-analysis.md](reference/io-analysis.md)
```

## Example 4: Domain-Specific Skill

### SKILL.md with Progressive Disclosure

```markdown
---
name: querying-analytics
description: Constructs and optimizes BigQuery analytics queries for business metrics. Use when analyzing user behavior, generating reports, or investigating data anomalies.
---

# Querying Analytics

## Common Queries

### Daily Active Users
\`\`\`sql
SELECT DATE(timestamp) as date,
       COUNT(DISTINCT user_id) as dau
FROM events.activity
WHERE DATE(timestamp) >= CURRENT_DATE() - 30
GROUP BY date
ORDER BY date DESC
\`\`\`

## Query Patterns

- **Cohort Analysis**: See [reference/cohorts.md](reference/cohorts.md)
- **Funnel Analysis**: See [reference/funnels.md](reference/funnels.md)
- **Revenue Metrics**: See [reference/revenue.md](reference/revenue.md)

## Optimization Tips

1. Partition by date when possible
2. Use approximate aggregation for large datasets
3. Materialize common subqueries
```

### reference/cohorts.md
```markdown
# Cohort Analysis Queries

## Weekly Cohort Retention

\`\`\`sql
WITH cohorts AS (
  SELECT user_id,
         DATE_TRUNC(MIN(DATE(first_seen)), WEEK) as cohort_week
  FROM users
  GROUP BY user_id
),
activities AS (
  SELECT user_id,
         DATE_TRUNC(DATE(timestamp), WEEK) as activity_week
  FROM events.activity
)
SELECT c.cohort_week,
       DATE_DIFF(a.activity_week, c.cohort_week, WEEK) as weeks_since,
       COUNT(DISTINCT a.user_id) as users
FROM cohorts c
JOIN activities a ON c.user_id = a.user_id
GROUP BY cohort_week, weeks_since
ORDER BY cohort_week, weeks_since
\`\`\`
```

## Example 5: Configuration Skill

### SKILL.md
```markdown
---
name: configuring-nginx
description: Configures Nginx web server for various scenarios including reverse proxy, SSL, and load balancing. Use when setting up web servers, configuring proxies, or optimizing web performance.
---

# Configuring Nginx

## Common Configurations

### Basic Static Site
\`\`\`nginx
server {
    listen 80;
    server_name example.com;
    root /var/www/html;
    index index.html;
}
\`\`\`

### Reverse Proxy
\`\`\`nginx
server {
    listen 80;
    server_name api.example.com;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
\`\`\`

## Setup Checklist

\`\`\`
Nginx Setup:
- [ ] Install: apt install nginx
- [ ] Configure: Edit /etc/nginx/sites-available/site
- [ ] Enable: ln -s ../sites-available/site ../sites-enabled/
- [ ] Test: nginx -t
- [ ] Reload: systemctl reload nginx
- [ ] Verify: curl -I localhost
\`\`\`

For advanced configurations:
- **SSL/TLS**: See [reference/ssl.md](reference/ssl.md)
- **Load Balancing**: See [reference/load-balancing.md](reference/load-balancing.md)
- **Caching**: See [reference/caching.md](reference/caching.md)
```