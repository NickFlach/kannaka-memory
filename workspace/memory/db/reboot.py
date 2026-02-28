#!/usr/bin/env python3
"""
Context reboot script - outputs compact summary for fresh context injection
"""
import sqlite3
from datetime import datetime, timedelta

DB_PATH = r"C:\Users\nickf\.openclaw\workspace\memory\kannaka.db"

def reboot_summary():
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    
    print("=== KANNAKA MEMORY REBOOT ===")
    print(f"Timestamp: {datetime.now()}")
    
    # Get working memory ordered by priority
    print("\n## WORKING MEMORY:")
    cursor.execute('''
        SELECT key, value, category, priority, updated_at
        FROM working_memory 
        WHERE expires_at IS NULL OR expires_at > datetime('now')
        ORDER BY priority DESC, updated_at DESC
    ''')
    wm_results = cursor.fetchall()
    for row in wm_results:
        key, value, category, priority, updated = row
        print(f"• {key}: {value} [{category}, p{priority}]")
    
    # Get recent events (last 2 hours)
    two_hours_ago = (datetime.now() - timedelta(hours=2)).isoformat()
    print(f"\n## RECENT EVENTS (since {two_hours_ago[-8:-3]}):")
    cursor.execute('''
        SELECT timestamp, category, event_type, subject, detail
        FROM events 
        WHERE timestamp > ?
        ORDER BY timestamp DESC
        LIMIT 10
    ''', (two_hours_ago,))
    event_results = cursor.fetchall()
    for row in event_results:
        timestamp, category, event_type, subject, detail = row
        subject_str = f" [{subject}]" if subject else ""
        detail_short = detail[:100] + "..." if len(detail) > 100 else detail
        time_str = timestamp[-8:-3] if len(timestamp) >= 8 else timestamp
        print(f"• {time_str} {category}.{event_type}{subject_str}: {detail_short}")
    
    # Get key entities
    print("\n## KEY ENTITIES:")
    cursor.execute('''
        SELECT name, type, attributes
        FROM entities
        WHERE type IN ('person', 'project') 
        ORDER BY type, name
        LIMIT 10
    ''')
    entity_results = cursor.fetchall()
    for row in entity_results:
        name, type, attributes = row
        attrs = ""
        if attributes:
            attrs = attributes[:50] + "..." if len(attributes) > 50 else attributes
        print(f"• {name} ({type}): {attrs}")
    
    # Get recent lessons
    print("\n## RECENT LESSONS:")
    cursor.execute('''
        SELECT lesson, category, learned_from
        FROM lessons
        ORDER BY created_at DESC
        LIMIT 5
    ''')
    lesson_results = cursor.fetchall()
    for row in lesson_results:
        lesson, category, learned_from = row
        from_str = f" (from: {learned_from})" if learned_from else ""
        print(f"• [{category}] {lesson}{from_str}")
    
    print("\n=== END REBOOT SUMMARY ===")
    
    conn.close()

if __name__ == "__main__":
    reboot_summary()