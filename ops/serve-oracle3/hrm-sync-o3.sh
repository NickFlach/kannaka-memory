#!/bin/bash
# ADR-0042 Phase 4 — sync the primary HRM to oracle3 for the redundant recall
# responder. rsync writes a temp file and atomically renames, so the O3 serve's
# native mtime watch (#565) sees one clean update and restart-to-reloads.
exec rsync -az -e "ssh -i /home/opc/.ssh/o3-sync -o StrictHostKeyChecking=accept-new" \
  /home/opc/.kannaka/kannaka.hrm opc@10.0.0.65:/home/opc/.kannaka/kannaka.hrm
