<template>
  <b-card class="p-3">
    <b-form @submit="login">
      <b-form-input
        class="my-3"
        type="email"
        placeholder="email"
        v-model="email"
        autofocus
        required
      ></b-form-input>
      <b-form-input
        class="my-3"
        placeholder="password"
        type="password"
        v-model="password"
        required
      ></b-form-input>
      <b-checkbox> Remember Me</b-checkbox>
      <b-button variant="secondary" class="my-5 mx-auto" @click="signup">Signup</b-button>
      <b-button type="submit" variant="primary" class="my-5 mx-1">Login</b-button>
    </b-form>
    <div>{{ message }}</div>
  </b-card>
</template>
<script lang="ts">
import Vue from 'vue';
import { useUserStore } from '~/stores/user';
import { useGlobalStore } from '~/stores/global';
import { mapState } from 'pinia';
import router from '../route';
export default {
  data: () => ({
    email: '',
    password: '',
    userStore: useUserStore(),
  }),
  computed: {
    ...mapState(useGlobalStore, { message: 'message' }),
  },
  watch: {},
  //   created() {},
  methods: {
    login(event: Event) {
      event.preventDefault();
      this.userStore.login(this.email, this.password);
    },
    signup() {
      router.push('/signup');
    },
  },
};
</script>