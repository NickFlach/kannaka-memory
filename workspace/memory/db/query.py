#!/usr/bin/env python3
"""
General query helper for Kannaka memory database
"""
import sqlite3
import sys

DB_PATH = r"C:\Users\nickf\.openclaw\workspace\memory\kannaka.db"

def run_query(sql):
    if not sql:
        print("Usage: python query.py 'SELECT * FROM working_memory;'")
        sys.exit(1)
    
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    
    try:
        cursor.execute(sql)
        if sql.strip().upper().startswith('SELECT'):
            results = cursor.fetchall()
            for row in results:
                print('\t'.join(str(x) if x is not None else '' for x in row))
        else:
            conn.commit()
            print('Query executed successfully')
    except Exception as e:
        print(f'Error: {e}')
        sys.exit(1)
    finally:
        conn.close()

if __name__ == "__main__":
    if len(sys.argv) != 2:
        run_query(None)
    else:
        run_query(sys.argv[1])