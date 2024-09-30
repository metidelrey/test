import { defineStore } from 'pinia';

interface State {
  message: string;
}

export const useGlobalStore = defineStore('global', {
  state(): State {
    return {
      message: '',
    };
  },
  actions: {
    setMessage(message: string) {
      this.message = message;
    },
  },
});
