#!/bin/bash

# TinyClaw Service Management Script
# Supports systemd and launchd

set -e

# Detect platform
PLATFORM="$(uname -s)"
SERVICE_NAME="tiny_claw"
EXEC_USER="${USER}"
BINARY_PATH="${HOME}/.cargo/bin/tiny_claw"
LOG_DIR="${HOME}/.local/share/tiny_claw/logs"
CONFIG_DIR="${HOME}/.config/tiny_claw"

install_systemd() {
    cat > "/tmp/${SERVICE_NAME}.service" << EOF
[Unit]
Description=TinyClaw AI Agent Gateway
After=network.target

[Service]
Type=simple
User=$EXEC_USER
ExecStart=$BINARY_PATH
Restart=on-failure
RestartSec=5
StandardOutput=append:$LOG_DIR/stdout.log
StandardError=append:$LOG_DIR/stderr.log

[Install]
WantedBy=multi-user.target
EOF

    echo "Installing systemd service..."
    sudo mv "/tmp/${SERVICE_NAME}.service" "/etc/systemd/system/"
    sudo systemctl daemon-reload
    sudo systemctl enable "$SERVICE_NAME"
    echo "Service installed. Use 'systemctl start $SERVICE_NAME' to start."
}

install_launchd() {
    mkdir -p "${HOME}/Library/LaunchAgents"
    
    cat > "${HOME}/Library/LaunchAgents/com.tiny_claw.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.tiny_claw</string>
    <key>ProgramArguments</key>
    <array>
        <string>$BINARY_PATH</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>$LOG_DIR/stdout.log</string>
    <key>StandardErrorPath</key>
    <string>$LOG_DIR/stderr.log</string>
</dict>
</plist>
EOF

    echo "Installing launchd service..."
    launchctl load "${HOME}/Library/LaunchAgents/com.tiny_claw.plist" 2>/dev/null || true
    echo "Service installed. Use 'launchctl start com.tiny_claw' to start."
}

uninstall() {
    if [ "$PLATFORM" = "Linux" ]; then
        echo "Removing systemd service..."
        sudo systemctl stop "$SERVICE_NAME" 2>/dev/null || true
        sudo systemctl disable "$SERVICE_NAME" 2>/dev/null || true
        sudo rm -f "/etc/systemd/system/${SERVICE_NAME}.service"
        sudo systemctl daemon-reload
    elif [ "$PLATFORM" = "Darwin" ]; then
        echo "Removing launchd service..."
        launchctl unload "${HOME}/Library/LaunchAgents/com.tiny_claw.plist" 2>/dev/null || true
        rm -f "${HOME}/Library/LaunchAgents/com.tiny_claw.plist"
    fi
    echo "Service uninstalled."
}

case "$1" in
    install)
        mkdir -p "$LOG_DIR"
        mkdir -p "$CONFIG_DIR"
        
        if [ "$PLATFORM" = "Linux" ]; then
            install_systemd
        elif [ "$PLATFORM" = "Darwin" ]; then
            install_launchd
        else
            echo "Unsupported platform: $PLATFORM"
            exit 1
        fi
        ;;
    uninstall)
        uninstall
        ;;
    *)
        echo "Usage: $0 {install|uninstall}"
        exit 1
        ;;
esac
