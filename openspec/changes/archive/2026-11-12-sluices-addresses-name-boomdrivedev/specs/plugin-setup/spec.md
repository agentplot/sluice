## MODIFIED Requirements

### Requirement: The herdr plugin is installed only on the person's word

The setup skill SHALL write nothing to herdr's file where herdr is
missing, older than 0.9.3, or has no plugin `agentplot.sluice`
installed. It SHALL offer the plugin's install and run it only on the
person's word.

#### Scenario: The plugin is not installed

- **WHEN** `herdr plugin list --plugin agentplot.sluice --json` lists
  no plugin
- **THEN** the skill offers `herdr plugin install
  boomdrivedev/sluice/plugins/herdr`, runs it only on the person's word,
  and writes nothing to herdr's file until the plugin is installed

