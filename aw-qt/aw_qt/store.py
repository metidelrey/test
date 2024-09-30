from .evnets import EventDetail, EventTypes
from .eventQueue import event_queue
class Store():
    def __init__(self) -> None:
        self.team_id = None
        self.token = None
        self.is_logged_in = None
        self.is_started = None
    
    def update_team_id(self, team_id:int):
        self.team_id = team_id
        
    def update_token(self, token:str):
        self.token = token
    
    def update_login_state(self, state:bool):
        self.is_logged_in = state
        
    def update_activity_state(self, state:bool):
        self.is_started = state
        
state_management = Store()

def observe(event: EventDetail):
    if(event.type == EventTypes.SUCCESSFUL_LOGIN):
        state_management.update_token(event.data)
        state_management.update_login_state(True)
    elif(event.type == EventTypes.START_ACTIVITY):
        state_management.update_team_id(event.data)
        state_management.update_activity_state(True)
    elif(event.type == EventTypes.STOP_ACTIVITY):
        state_management.update_activity_state(False)
    elif(event.type == EventTypes.RESET_ACTIVITY):
        state_management.update_team_id(event.data)
        state_management.update_activity_state(False)
    elif(event.type == EventTypes.LOGOUT):
        state_management.update_login_state(False)
        state_management.update_activity_state(False)
event_queue.subscribe(on_next=observe)
    