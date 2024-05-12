#!/bin/bash

tmux new-session -d -s server

tmux send-keys -t server "/home/admin/website/raspberry-website/website" C-m
