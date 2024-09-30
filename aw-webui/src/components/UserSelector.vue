<template>
  <b-table
    show-empty
    striped
    :hover="false"
    :items="users"
    :fields="fields"
    :empty-text="'No users found'"
  >
    <template #cell(action)="data">
      <!-- <b-button variant="primary" @click="editItem(data.item)"> Edit </b-button> -->
      <b-checkbox @change="selectItem(data.item)"></b-checkbox>
    </template>
  </b-table>
</template>

<script>
import { useUserStore } from '@/stores/user';
export default {
  props: ['excludeIds'],
  data: () => ({
    fields: [
      { key: 'action', label: '' },
      { key: 'name', label: 'name' },
      { key: 'lastname', label: 'lastname' },
      { key: 'email', label: 'email' },
    ],
    users: [],
    userStore: useUserStore(),
    selectedMembers: [],
  }),
  mounted() {
    this.getAllUsers();
  },
  methods: {
    async getAllUsers() {
      const users = await this.userStore.getAllUsers();
      this.users = users.filter(user => !this.excludeIds.includes(user.id));
    },
    selectItem(item) {
      const itemIndex = this.selectedMembers.lastIndexOf(item.id);

      if (itemIndex === -1) {
        this.selectedMembers.push(item.id);
      } else {
        this.selectedMembers.splice(itemIndex, 1);
      }
      this.$emit('selected-members-changed', this.selectedMembers);
    },
  },
};
</script>

<style>
</style>