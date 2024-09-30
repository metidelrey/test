<template>
  <div>
    <b-row class="d-flex justify-content-start align-items-center input__container">
      <b-form-input class="my-3" placeholder="App Name" v-model="appName" autofocus></b-form-input>
      <b-button :disabled="appName.length === 0" @click="addNewApp" size="sm" variant="outline-primary" class="plus__button">
        <icon name="plus" scale="1"></icon>
      </b-button>
    </b-row>
    <b-row class="d-flex justify-content-start align-items-center input__container">
      <chip v-for="app of apps" :key="app" :label="app" :removable="true" @remove="removeAppName"></chip>
    </b-row>
  </div>
</template>

<script>
import 'vue-awesome/icons/plus';
import { useTeamStore } from '@/stores/team';
import Chip from './Chip.vue';
export default {
  props: ['teamId'],
  components: { 'chip': Chip },
  data: () => ({
    appName: '',
    apps: [],
    teamStore: useTeamStore(),
  }),
  mounted() {
    this.getAllApps();
  },
  methods: {
    async getAllApps() {
      this.apps = await this.teamStore.getConfiguration(this.teamId);
    },
    addNewApp(){
      this.apps.push(this.appName)
      this.appName = ''
      this.$emit("selectedAppsChanged", this.apps)
    },
    removeAppName(appName){
      const index = this.apps.lastIndexOf(appName)
      if(index > -1){
        this.apps.splice(index,1)
        this.$emit("selectedAppsChanged", this.apps)
      }
    }
  },
};
</script>

<style lang="scss" scoped>
.input__container {
  flex-wrap: nowrap;
  padding: 0px 10px;
  gap: 10px;
}
</style>