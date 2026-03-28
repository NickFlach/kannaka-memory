#!/bin/bash
# Automated backup of NATS JetStream data (QS-8, #59)
# Add to crontab: 0 3 * * * /opt/kannaka/scripts/backup-jetstream.sh
#
# Backs up:
#   - JetStream data directory
#   - NATS server configuration
#
# Retention: 7 daily backups

set -euo pipefail

JETSTREAM_DIR="/var/lib/nats/jetstream"
BACKUP_BASE="/var/backups/nats"
NATS_CONFIG="/etc/nats/nats-server.conf"
RETENTION_DAYS=7
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="${BACKUP_BASE}/${TIMESTAMP}"

echo "[$(date)] Starting JetStream backup..."

# Create backup directory
mkdir -p "${BACKUP_DIR}"

# Backup JetStream data
if [ -d "${JETSTREAM_DIR}" ]; then
    tar czf "${BACKUP_DIR}/jetstream-${TIMESTAMP}.tar.gz" \
        -C "$(dirname ${JETSTREAM_DIR})" \
        "$(basename ${JETSTREAM_DIR})"
    echo "  JetStream data backed up: ${BACKUP_DIR}/jetstream-${TIMESTAMP}.tar.gz"
else
    echo "  WARNING: JetStream directory not found: ${JETSTREAM_DIR}"
fi

# Backup config
if [ -f "${NATS_CONFIG}" ]; then
    cp "${NATS_CONFIG}" "${BACKUP_DIR}/nats-server.conf"
    echo "  Config backed up"
fi

# Cleanup old backups
find "${BACKUP_BASE}" -maxdepth 1 -type d -mtime +${RETENTION_DAYS} -exec rm -rf {} \;
echo "  Old backups cleaned (retention: ${RETENTION_DAYS} days)"

# Report size
TOTAL_SIZE=$(du -sh "${BACKUP_DIR}" | cut -f1)
echo "[$(date)] Backup complete. Size: ${TOTAL_SIZE}"
