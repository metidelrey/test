from enum import Enum

class EventTypes(Enum):
    SUCCESSFUL_LOGIN = 1
    FAILED_LOGIN = 2
    TOKEN_EXPIRED = 3
    START_ACTIVITY = 4
    STOP_ACTIVITY = 5
    LOGOUT = 6
    RESET_ACTIVITY = 7


class EventDetail():
    def __init__(self, type:EventTypes, data:any) -> None:
        self.type = type
        self.data = data