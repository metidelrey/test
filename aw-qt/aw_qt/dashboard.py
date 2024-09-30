from PyQt6 import QtCore
from PyQt6.QtGui import QIcon, QShowEvent
from PyQt6.QtWidgets import (
    QPushButton,
    QLabel,
    QVBoxLayout,
    QDialog,
    QComboBox,
    QHBoxLayout
)
from .chip import ChipButton
import sys
from .client import client
from .datastore import DataStore
from .evnets import EventDetail, EventTypes
from enum import Enum
from .eventQueue import event_queue
from .store import state_management

class ActivityState(Enum):
    STARTED = 1
    STOPPED = 2
    
class Team:
    def __init__(self, team) -> None:
        self.id = team['id']
        self.name = team['name']
        self.description = team['description']

class DashboardPage(QDialog):
    def __init__(self, url):
        super().__init__()
        self.client = client(url)
        self.datastore = DataStore()
        self.event_queue = event_queue
        self.team_id = None
        self.layout = QVBoxLayout()
        self.activity_state = ActivityState.STOPPED
        
        teams_labels = QLabel("Teams:")
        self.selector = QComboBox()
        self.selector.currentIndexChanged.connect(self.load_configuration)
        self.layout.addWidget(teams_labels)
        self.layout.addWidget(self.selector)
        
        # configuration_label = QLabel("Configurations")
        apps_label = QLabel("Apps: ")
        self.apps_layout = QHBoxLayout()
        privacy_lable = QLabel('Note: Only the above apps are being tracked for privacy purposes.')
        privacy_lable.setStyleSheet('font-size: 10px; color: gray;')
        
        # self.layout.addWidget(configuration_label)
        self.layout.addSpacing(10)
        self.layout.addWidget(apps_label)
        self.layout.addLayout(self.apps_layout)
        self.layout.addWidget(privacy_lable)
        self.layout.addStretch()
        
        self.button = QPushButton("Start")
        self.button.setDisabled(True)
        self.signout_button = QPushButton("Logout")
        self.button.clicked.connect(self.change_activity_state)
        self.signout_button.clicked.connect(self.logout)
        self.layout.addWidget(self.button)
        # self.layout.addWidget(self.signout_button)
        
        self.setLayout(self.layout)
        self.setGeometry(100, 100, 300, 300)

    def change_activity_state(self):
        if(self.activity_state == ActivityState.STOPPED):
            self.button.setText("Stop")
            self.activity_state = ActivityState.STARTED
            self.event_queue.on_next(EventDetail(EventTypes.START_ACTIVITY, self.team_id))
        elif(self.activity_state == ActivityState.STARTED):
            self.button.setText("Start")
            self.activity_state = ActivityState.STOPPED
            self.event_queue.on_next(EventDetail(EventTypes.STOP_ACTIVITY, None))
    
    def logout(self):
        self.event_queue.on_next(EventDetail(EventTypes.STOP_ACTIVITY, None))
        self.event_queue.on_next(EventDetail(EventTypes.LOGOUT, None))
        
    def getTeams(self):
        response = self.client.get_teams()
        if response.status_code == 200:
            for team in response.json():
                self.selector.addItem(team['name'],Team(team))
    
    def load_configuration(self):
        self.clear_apps_layout()
        selected_team = self.selector.currentData()
        if(selected_team is None or selected_team.id is None):
            self.button.setDisabled(True)
            return
        else:
            self.button.setDisabled(False)
        self.team_id = selected_team.id
        if(self.activity_state == ActivityState.STARTED):
            self.event_queue.on_next(EventDetail(EventTypes.RESET_ACTIVITY, self.team_id))
        configs = self.client.get_team_configuration(self.team_id)
        for app in configs:
            self.apps_layout.addWidget(ChipButton(app), alignment=QtCore.Qt.AlignmentFlag.AlignLeft)
        self.apps_layout.addStretch()
            
                
    def clear_apps_layout(self):
        while self.apps_layout.count():
            widget = self.apps_layout.itemAt(0).widget()
            if widget is not None:
                widget.deleteLater()
            self.apps_layout.removeItem(self.apps_layout.itemAt(0))
        
    
    def showEvent(self, a0: QShowEvent) -> None:
        super().showEvent(a0)
        self.getTeams()
        self.setStates()
        
    def setStates(self):
        if(state_management.is_started):
            self.button.setText("Stop")
            self.activity_state = ActivityState.STARTED
        
            
            
        
            
   