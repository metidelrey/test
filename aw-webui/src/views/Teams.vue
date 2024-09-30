<template>
  <div class="teams__container">
    <h3>Teams</h3>

    <template v-if="teams.length === 0">
      <div class="no-teams__container">
        <img class="no-team__image" src="group.png" />
        <div class="no-team__message">No team Found</div>
      </div>
    </template>
    <template v-else>
      <div class="cards__container">
        <team-card v-for="team of teams" :key="team.id" :team="team"></team-card>
      </div>
    </template>
    <b-button @click="showTeamDialog" class="add__button">
      <icon name="plus"></icon>
      Add a Team
    </b-button>
    <b-modal title="Team" ref="addTeam" @ok="handleOk">
      <b-form>
        <b-form-input
          class="my-3"
          placeholder="name"
          v-model="selectedTeam.name"
          autofocus
          required
        ></b-form-input>
        <b-form-textarea
          class="my-3"
          placeholder="description"
          v-model="selectedTeam.description"
        ></b-form-textarea>
      </b-form>
    </b-modal>
  </div>
</template>

<script lang="ts">
import 'vue-awesome/icons/plus';
import { mapState } from 'pinia';
import { useTeamStore } from '~/stores/team';
import TeamCardVue from '@/components/TeamCard.vue';

export default {
  components: {
    'team-card': TeamCardVue,
  },
  data: () => ({
    selectedTeam: {
      id: -1,
      name: '',
      description: '',
    },
    teamStore: useTeamStore(),
  }),
  computed: {
    ...mapState(useTeamStore, ['teams']),
  },
  mounted() {
    this.teamStore.getTeams();
  },
  methods: {
    showTeamDialog() {
      this.$refs.addTeam.show();
    },
    handleOk(e) {
      this.teamStore.addTeam(this.selectedTeam);
    },
  },
};
</script>
<style lang="scss" scoped>
.teams__container {
  height: 80vh;
  overflow-y: auto;
}
.no-teams__container {
  display: flex;
  height: 70vh;
  margin: auto;
  justify-content: center;
  align-items: center;
  flex-direction: column;
  .no-team__image {
    width: 300px;
    height: auto;
  }
  .no-team__message {
    font-size: 1.3em;
  }
}

.add__button {
  margin: auto;
}

.cards__container {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}
</style>
