#!/usr/bin/env bash
set +x

# kill all child processes when exiting
trap "kill -- -$$" SIGINT SIGTERM EXIT

# test the setup using wsl as the ocr and main server
# with windows hosting the screen shotting server
WINDOWS_HOST=$(nslookup $(hostname) | head -n 1 | awk -F' ' '{ print $2 }')
export SCREEN_ENDPOINT=http://${WINDOWS_HOST}:3000/capture?window_title=Spelunky
export OCR_PORT=3001
export OCR_ENDPOINT=http://127.0.0.1:${OCR_PORT}/ocr

# start spenlunky
(spelunky 2>&1 > /dev/null & sleep 1)
# start ocr server
PORT=${OCR_PORT} cargo r --bin ocr &
# start main handler
GAME_PID=$(ps -a | grep spelunky | awk -F' ' '{ print $1 }')
cargo r --bin game-poc ${GAME_PID}
