<template>
  <b-card class="p-3">
    <b-form @submit="signup">
      <b-form-input class="my-3" placeholder="name" v-model="model.name" required></b-form-input>
      <b-form-input
        class="my-3"
        placeholder="lastname"
        v-model="model.lastname"
        required
      ></b-form-input>
      <b-form-input
        class="my-3"
        placeholder="username"
        v-model="model.username"
        required
      ></b-form-input>
      <b-form-input
        class="my-3"
        placeholder="email"
        type="email"
        v-model="model.email"
        required
      ></b-form-input>
      <b-form-input
        class="my-3"
        placeholder="password"
        type="password"
        v-model="model.password"
        required
      ></b-form-input>
      <b-checkbox> Remember Me</b-checkbox>
      <b-button variant="secondary" class="my-5 mx-auto" @click="login">Login</b-button>
      <b-button type="submit" variant="primary" class="my-5 mx-1">Signup</b-button>
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
    model: {
      name: '',
      lastname: '',
      username: '',
      email: '',
      password: '',
    },
    userStore: useUserStore(),
  }),
  computed: {
    ...mapState(useGlobalStore, { message: 'message' }),
  },
  watch: {},
  //   created() {},
  methods: {
    signup(event: Event) {
      event.preventDefault();
      this.userStore.signup(this.model);
    },
    login() {
      router.push('/login');
    },
  },
};
</script>
