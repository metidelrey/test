import sqlite3
class DataStore():
    def __init__(self) -> None:
        self.conn = sqlite3.connect('ttclient.db')
        cursor = self.conn.cursor()
        cursor.execute('''
                CREATE TABLE IF NOT EXISTS credential (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    token TEXT NOT NULL
                )
                ''')
    def saveToken(self, token) -> None:
        cursor = self.conn.cursor()
        cursor.execute(f"INSERT INTO credential (token) VALUES ('{token}')")
        self.conn.commit()
    
    def readToken(self) -> str:
        cursor = self.conn.execute(f"SELECT token FROM credential LIMIT 1")
        return cursor.fetchone()
        
        