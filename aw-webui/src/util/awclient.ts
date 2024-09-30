import { AWClient, AWReqOptions, IBucket, IEvent } from 'aw-client';

import { useSettingsStore } from '~/stores/settings';
import { useGlobalStore } from '~/stores/global';

interface GetEventsOptions {
  start?: Date;
  end?: Date;
  limit?: number;
  teamId?: number;
}

export class CustomAwClient extends AWClient {
  constructor(clientName: string, options?: AWReqOptions) {
    super(clientName, options);
    this.setErrorHandler();
  }

  setErrorHandler() {
    this.req.interceptors.response.use(
      res => {
        return Promise.resolve(res);
      },
      rej => {
        const globalStore = useGlobalStore();
        globalStore.setMessage(rej.response.data.message);
        return Promise.resolve(rej.response);
      }
    );
  }

  login(email: string, password: string) {
    return this.req.post('/user/login', { email, password: password });
  }

  signup(user) {
    this.req.post('/user/signup', user);
  }

  setToken(token: string) {
    this.req.defaults.headers['Authorization'] = `Bearer ${token}`;
  }

  clearToken() {
    delete this.req.defaults.headers['Authorization'];
  }

  getUser() {
    return this.req.get('/user/getuser');
  }

  getAllUsers() {
    return this.req.get('/user/users');
  }

  getTeams() {
    return this.req.get('/teams');
  }

  addTeam(team) {
    return this.req.post('/teams', team);
  }

  editTeam(team) {
    return this.req.put('/teams', team);
  }

  getTeam(teamId: number) {
    return this.req.get(`/teams/team/${teamId}`);
  }

  addMembers(teamId: number, members: number[]) {
    return this.req.post(`/teams/${teamId}/members`, members);
  }

  removeMember(teamId: number, memberId: number) {
    return this.req.delete(`/teams/${teamId}/member/${memberId}`);
  }

  getConfiguration(teamId: number) {
    return this.req.get(`/teams/configuration/${teamId}`);
  }

  addConfiguration(teamId: number, configuration: any) {
    return this.req.post(`/teams/${teamId}/configuration`, configuration);
  }

  override async getEvents(bucketId: string, params?: GetEventsOptions): Promise<IEvent[]> {
    const response = await this.req.get(`/0/buckets/${bucketId}/events?start=${params.start}&end=${params.end}&limit=${params.limit}`)
    return response.data;
  }

  async getUserBuckets(userId: number): Promise<{ [bucketId: string]: IBucket; }> {
    const response = await this.req.get(`/0/buckets/${userId}`);
    return response.data;
  }

  async getBucket(bucketId: number): Promise<IBucket> {
    const response = await this.req.get(`/0/buckets/${bucketId}/info`);
    return response.data;
  }

  async getUserEvents(bucketId: string, params?: GetEventsOptions): Promise<IEvent[]> {
    const response = await this.req.get(`/0/buckets/${bucketId}/events?start=${params.start}&end=${params.end}&limit=${params.limit}&team_id=${params.teamId}`)
    return response.data;
  }
}

let _client: CustomAwClient | null;

export function createClient(force?: boolean): CustomAwClient {
  let baseURL = '';

  const production = typeof PRODUCTION !== 'undefined' && PRODUCTION;

  // If running with `npm node dev`, use testing server as origin.
  // Works since CORS is enabled by default when running `aw-server --testing`.
  if (!production) {
    const aw_server_url = typeof AW_SERVER_URL !== 'undefined' && AW_SERVER_URL;
    baseURL = aw_server_url || 'http://127.0.0.1:5666';
  }

  if (!_client || force) {
    _client = new CustomAwClient('aw-webui', {
      testing: !production,
      baseURL,
    });
  } else {
    throw 'Tried to instantiate global AWClient twice!';
  }
  return _client;
}

export function configureClient(): void {
  const settings = useSettingsStore();
  _client.req.defaults.timeout = 1000 * settings.requestTimeout;
}

export function getClient(): CustomAwClient {
  if (!_client) {
    throw 'Tried to get global AWClient before instantiating it!';
  }
  return _client;
}
