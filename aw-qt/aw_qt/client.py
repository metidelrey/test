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
class client():
    def __init__(self, url) -> None:
        self.url = url
        self.token = ''
        
    def login(self, email:str, passowrd:str):
        data = {}
        data['email'] = email
        data['password'] = passowrd
        response = requests.post(self.url+'/api/user/login', data=bytes(json.dumps(data), "utf8"))
        if(response.status_code == 200):
            self.token = "Bearer " + response.json()
        return response
    
    def get_teams(self):
        headers = {'Authorization': self.token}
        response = requests.get(self.url+'/api/teams/user', headers=headers)
        return response
    
    def get_team_configuration(self, team_id:int):
        headers = {'Authorization': self.token}
        # return ['chrome','firefox']
        response = requests.get(self.url+f'/api/teams/configuration/{team_id}', headers=headers)
        if(response.status_code == 200):
            return response.json()