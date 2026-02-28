#!/usr/bin/env python3
"""
Query entity and relationships
"""
import sqlite3
import sys

DB_PATH = r"C:\Users\nickf\.openclaw\workspace\memory\kannaka.db"

def get_entity(name):
    if not name:
        print("Usage: python get-entity.py 'Nick'")
        sys.exit(1)
    
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    
    print(f"=== ENTITY: {name} ===")
    
    # Get the entity
    cursor.execute('SELECT * FROM entities WHERE name = ?', (name,))
    entity = cursor.fetchone()
    
    if not entity:
        print("Entity not found")
        conn.close()
        return
    
    id, name, type, attributes, created_at, updated_at = entity
    print(f"Type: {type}")
    print(f"Created: {created_at}")
    print(f"Updated: {updated_at}")
    if attributes:
        print(f"Attributes: {attributes}")
    
    # Get outgoing relationships
    print(f"\n--- Relationships FROM {name} ---")
    cursor.execute('''
        SELECT relation, to_entity, context
        FROM relationships 
        WHERE from_entity = ?
        ORDER BY relation
    ''', (name,))
    out_rels = cursor.fetchall()
    for rel in out_rels:
        relation, to_entity, context = rel
        context_str = f" ({context})" if context else ""
        print(f"• {name} --{relation}--> {to_entity}{context_str}")
    
    # Get incoming relationships
    print(f"\n--- Relationships TO {name} ---")
    cursor.execute('''
        SELECT from_entity, relation, context
        FROM relationships 
        WHERE to_entity = ?
        ORDER BY relation
    ''', (name,))
    in_rels = cursor.fetchall()
    for rel in in_rels:
        from_entity, relation, context = rel
        context_str = f" ({context})" if context else ""
        print(f"• {from_entity} --{relation}--> {name}{context_str}")
    
    conn.close()

if __name__ == "__main__":
    if len(sys.argv) != 2:
        get_entity(None)
    else:
        get_entity(sys.argv[1])