<template lang="pug">
#wrapper(v-if='loaded')
  aw-header

  .px-0.px-md-2(:class='{ container: !fullContainer, "container-fluid": fullContainer }')
    .aw-container.my-sm-3.p-3
      //- error-boundary
        //- user-satisfaction-poll
        //- new-release-notification(v-if="isNewReleaseCheckEnabled")
      router-view

  aw-footer
</template>

<script lang="ts">
import { useSettingsStore } from '~/stores/settings';
import { useServerStore } from '~/stores/server';
import { useUserStore } from '~/stores/user';
import { mapState } from 'pinia';
import { getClient } from './util/awclient';
// if vite is used, you can import css file as module
//import darkCssUrl from '../static/dark.css?url';
//import darkCssContent from '../static/dark.css?inline';

export default {
  data: function () {
    return {
      activityViews: [],
      isNewReleaseCheckEnabled: !process.env.VUE_APP_ON_ANDROID,
      loaded: false,
      userStore: useUserStore(),
    };
  },

  computed: {
    fullContainer() {
      return this.$route.meta.fullContainer;
    },
    ...mapState(useUserStore, { isLoggedIn: 'isLoggedIn' }),
    ...mapState(useUserStore, { userId: 'userId' }),
  },

  async beforeCreate() {
    // Get Theme From LocalStorage
    const settingsStore = useSettingsStore();
    await settingsStore.ensureLoaded();
    const theme = settingsStore.theme;
    // Check Application Mode (Light | Dark)
    if (theme !== null && theme === 'dark') {
      const method: 'link' | 'style' = 'link';

      if (method === 'link') {
        // Method 1: Create <link> Element
        // Create Dark Theme Element
        const themeLink = document.createElement('link');
        themeLink.href = '/dark.css'; // darkCssUrl
        themeLink.rel = 'stylesheet';
        // Append Dark Theme Element If Selected Mode Is Dark
        theme === 'dark' ? document.querySelector('head').appendChild(themeLink) : '';
      } else {
        // Not supported for Webpack due to not supporting ?inline import in a cross-compatible way (afaik)
        // Method 2: Create <style> Element
        //const style = document.createElement('style');
        //style.innerHTML = darkCssContent;
        //theme === 'dark' ? document.querySelector('head').appendChild(style) : '';
      }
    }
    this.loaded = true;
  },

  mounted: async function () {
    if (this.isLoggedIn) {
      this.userStore.setToken();
    }
  },
};
</script>
