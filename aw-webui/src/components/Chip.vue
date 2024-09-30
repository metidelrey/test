<template>
  <div @click="removeChip" class="chip">
    <b-badge class="badge">{{ label }}</b-badge>
    <icon @click="removeChip" v-if="removable" class="icon" name="times"></icon>
  </div>
</template>

<script>
import 'vue-awesome/icons/times';

export default {
  props: {
    label: {
      type: String,
      required: true,
    },
    removable: {
      type: Boolean,
      required: false
    }
  },
  methods: {
    removeChip() {
      this.$emit('remove', this.label);
    },
  },
};
</script>

<style lang="scss" scoped>
.chip {
  display: inline-flex;
  align-items: center;
  background-color: var(--primary);
  border-radius: 16px;
  padding: 0.4em 0.6em;

  .badge {
    background-color: var(--primary);
    color: #f1f1f1;
    user-select: none;
  }

  .icon {
    width: 10px;
    color: #f1f1f1;
    cursor: pointer;
  }
}
</style>