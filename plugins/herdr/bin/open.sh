#!/bin/sh
# Open one of the plugin's panes: status, dashboard or setup.
exec "${HERDR_BIN_PATH:-herdr}" plugin pane open --plugin agentplot.sluice --entrypoint "$1"
