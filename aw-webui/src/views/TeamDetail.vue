<template>
  <div class="team-detail__container">
    <h3 class="name__text">Team: {{ team.name }}</h3>
    <h4>Configurations:</h4>
    <b-row class="d-flex justify-content-start align-items-center apps__container">
      Apps:
      <template v-if="apps.length > 0">
        <Chip variant="secondary" :label="app" :removable="false" v-for="app of apps" :key="app">
        </Chip>
      </template>
      <template v-else>
        No app found.
      </template>
    </b-row>
    <b-button variant="secondary" class="my-5 mx-1" @click="editConfiguration">
      <icon name="pen"></icon>
      Edit Configuration
    </b-button>
    <h4>Members:</h4>
    <div>
      <b-table show-empty striped hover :items="members" :fields="fields" :empty-text="'No members found'">
        <template #cell(actions)="data">
          <b-row class="d-flex justify-content-start align-items-center actions__container">
            <b-button @click="viewActivity(data.item)" size="sm" variant="primary">
              <icon name="eye"></icon>
              Activity
            </b-button>
            <b-button @click="deleteItem(data.item)" size="sm" variant="danger">
              <icon name="trash"></icon>
              Delete
            </b-button>
          </b-row>
        </template>
      </b-table>
    </div>
    <b-button variant="primary" class="my-5 mx-1" @click="addMember">
      <icon name="plus"></icon>
      Add member
    </b-button>
    <b-modal title="Add Member" ref="addMember" @ok="addNewMembers">
      <UserSelector :excludeIds="members.map(member => member.user_id)"
        @selected-members-changed="selectedMembersChanged">
      </UserSelector>
    </b-modal>
    <b-modal title="Edit Configuration" ref="configuration" @ok="updateConfiguration">
      <TeamConfigSelector :team-id="teamId" @selectedAppsChanged="selectedAppsChanged"></TeamConfigSelector>
    </b-modal>
  </div>
</template>

<script>
import 'vue-awesome/icons/trash';
import 'vue-awesome/icons/eye';
import 'vue-awesome/icons/plus';
import 'vue-awesome/icons/pen';
import { useTeamStore } from '@/stores/team';
import UserSelector from '@/components/UserSelector.vue';
import TeamConfigSelector from '@/components/TeamConfigSelector.vue';
import Chip from '@/components/Chip.vue';
export default {
  components: {
    UserSelector,
    TeamConfigSelector,
    Chip
  },
  data() {
    return {
      // Note `isActive` is left out and will not appear in the rendered table
      fields: ['name', 'lastname', 'email', { key: 'actions', label: 'actions' }],
      members: [
      ],
      apps: [],
      teamId: this.$route.params['id'],
      team: {},
      teamStore: useTeamStore(),
      selectedMembers: [],
      selectedApps: []
    };
  },
  mounted() {
    this.getTeam();
  },

  methods: {
    async getTeam() {
      this.team = await this.teamStore.getTeam(this.teamId);
      this.members = this.team.members;
      this.apps = this.team.apps;
    },

    addMember() {
      this.$refs['addMember'].show();
    },

    selectedMembersChanged(membersId) {
      this.selectedMembers = membersId;
    },

    async addNewMembers() {
      if (this.selectedMembers.length > 0) {
        await this.teamStore.addMembers(this.teamId, this.selectedMembers);
        this.getTeam();
      }
    },

    async deleteItem(item) {
      await this.teamStore.removeMember(this.teamId, item.id);
      this.getTeam();
    },

    viewActivity(item) {
      this.$router.push(`/user/${item.user_id}/${this.teamId}`)
    },

    selectedAppsChanged(apps) {
      this.selectedApps = apps
    },

    editConfiguration() {
      this.$refs['configuration'].show();
    },

    updateConfiguration() {
      this.teamStore.addConfiguration(this.teamId, this.selectedApps);
      this.getTeam();
    }
  },
};
</script>
<style lang="scss" scoped>
.team-detail__container {
  height: 80vh;
}

.name__text {
  margin-bottom: 50px;
}

.actions__container {
  gap: 5px;
}

.apps__container {
  gap: 5px;
  padding: 0 20px;
}
</style>