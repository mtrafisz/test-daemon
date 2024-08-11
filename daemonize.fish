#!/bin/fish

if test -z $argv[1]
    echo "Usage: $argv[1] {install|uninstall|status}"
    exit 1
end

if test $argv[1] = "install"
    cargo build --release
    sudo cp target/release/nfs-daemon -t /usr/local/bin
    sudo cp nfs-daemon.service -t /etc/systemd/system
    sudo mkdir -p /usr/local/share/nfs-daemon
    sudo systemctl daemon-reload
    sudo systemctl enable nfs-daemon
    sudo systemctl start nfs-daemon
else if test $argv[1] = "uninstall"
    sudo systemctl stop nfs-daemon
    sudo systemctl disable nfs-daemon
    sudo rm -f /etc/systemd/system/nfs-daemon.service
    sudo systemctl daemon-reload
    sudo rm -f /usr/local/bin/nfs-daemon
    sudo rm -rf /usr/local/share/nfs-daemon
else if test $argv[1] = "status"
    sudo systemctl status nfs-daemon
else
    echo "Usage: $argv[1] {install|uninstall|status}"
    exit 1
end
