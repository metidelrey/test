import requests
import json


def singleton(cls):
    instances = {}
    def get_instance(*args, **kwargs):
        if cls not in instances:
            instances[cls] = cls(*args, **kwargs)
        return instances[cls]
    return get_instance

@singleton
class LocalClient():
    def __init__(self, url) -> None:
        self.url = url
        self.token = ''
    def get_team_configuration(self, team_id:int):
        headers = {'Authorization': self.token}
        response = requests.get(self.url+f'/api/teams/configuration/{team_id}', headers=headers)
        if(response.status_code == 200):
            return response.json()