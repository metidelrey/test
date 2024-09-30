from PyQt6 import QtCore
from PyQt6.QtGui import QIcon
from PyQt6.QtWidgets import (
    QApplication,
    QMenu,
    QMessageBox,
    QPushButton,
    QSystemTrayIcon,
    QLabel,
    QVBoxLayout,
    QLineEdit,
    QWidget,
    QDialog
)
import sys
from .client import client
from .datastore import DataStore
from rx.subject import Subject
from .evnets import EventDetail, EventTypes
from .eventQueue import event_queue
class LoginPage(QDialog):
    def __init__(self, url):
        super().__init__()
        self.client = client(url)
        self.datastore = DataStore()
        self.event_queue = event_queue
        self.setGeometry(100, 100, 300, 150)
        
        # Create layout
        self.form_layout = QVBoxLayout()
        
        # Email Label and Input
        self.email_label = QLabel("Email:")
        self.email_input = QLineEdit()
        self.email_input.setPlaceholderText("Enter your email")
        
        # Password Label and Input
        self.password_label = QLabel("Password:")
        self.password_input = QLineEdit()
        self.password_input.setPlaceholderText("Enter your password")
        self.password_input.setEchoMode(QLineEdit.EchoMode.Password)

        # Create Login Button
        self.login_button = QPushButton("Login")
        self.login_button.clicked.connect(self.check_credentials)

        # Add Widgets to Layout
        self.form_layout.addWidget(self.email_label)
        self.form_layout.addWidget(self.email_input)
        self.form_layout.addSpacing(10)
        self.form_layout.addWidget(self.password_label)
        self.form_layout.addWidget(self.password_input)
        self.form_layout.addStretch()
        self.form_layout.addWidget(self.login_button)

        # Set the layout for the window
        self.setLayout(self.form_layout)

    def check_credentials(self):
        email = self.email_input.text()
        password = self.password_input.text()
        response = self.client.login(email,password)
        if(response.status_code == 200):
            self.datastore.saveToken(response.json())
            self.event_queue.on_next(EventDetail(EventTypes.SUCCESSFUL_LOGIN, response.json()))
        else:
            self.event_queue.on_next(EventDetail(EventTypes.FAILED_LOGIN, None))
        
            
   