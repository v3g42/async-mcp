#!/bin/bash
tee /tmp/mcp.fifo | ./target/debug/file_system | tee /tmp/mcp.fifo