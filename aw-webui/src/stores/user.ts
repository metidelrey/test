import { defineStore } from 'pinia';
import { getClient } from '~/util/awclient';
import route from '../route';
interface User {
  userId: string;
  teamIds: string[];
  roles: number[];
  isLoggedIn: boolean;
  token: string;
}

interface UserSignupModel {
  id: number;
  email: string;
  name: string;
  lastname: string;
  password: string;
}

export const useUserStore = defineStore('user', {
  state(): User {
    return (
      (JSON.parse(localStorage.getItem('user')) as User) || {
        isLoggedIn: false,
        teamIds: [],
        userId: '',
        roles: [],
        token: '',
      }
    );
  },
  actions: {
    async login(email: string, password: string) {
      const client = getClient();
      const response = await client.login(email, password);
      if (response.status === 200) {
        const token = response.data;
        this.setLoginState(token);
        this.setToken();
        this.persistToLocalStorage();
        this.getUser();
        route.push('/teams');
      }
    },

    setToken() {
      const client = getClient();
      client.setToken(this.token);
    },

    signup(user: UserSignupModel) {
      const client = getClient();
      client.signup(user);
    },

    setLoginState(token: string) {
      this.token = token;
      this.isLoggedIn = true;
    },

    logout() {
      this.token = '';
      this.isLoggedIn = false;
      this.clearLocalStorage();
    },

    persistToLocalStorage() {
      localStorage.setItem(
        'user',
        JSON.stringify({
          isLoggedIn: this.isLoggedIn,
          roles: this.roles,
          teamIds: this.teamIds,
          token: this.token,
          userId: this.userId,
        } as User)
      );
    },
    clearLocalStorage() {
      localStorage.removeItem('user');
    },

    getUser() {
      const client = getClient();
      return client.getUser();
    },

    async getAllUsers() {
      const client = getClient();
      const response = await client.getAllUsers();
      return response.data;
    },
  },
});
