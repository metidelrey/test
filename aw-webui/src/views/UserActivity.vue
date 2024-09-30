<template>
    <b-tabs content-class="mt-3">
        <!-- <b-tab title="Activity">
            <Activity></Activity>
        </b-tab> -->
        <b-tab title="Timeline">
            <timeline :bucketIds="bucketIds" :teamId="teamId"></timeline>
        </b-tab>
    </b-tabs>
</template>
<script>
import { getClient } from "@/util/awclient";
import Activity from "./activity/Activity.vue";
import Timeline from "./Timeline.vue";
export default {
    components: {
        // Activity,
        Timeline
    },
    data() {
        return {
            userId: this.$route.params['userId'],
            teamId: this.$route.params['teamId'],
            buckets: [],
            bucketIds: [],
            loaded: false
        }
    },

    created() {
        this.getBuckets();
    },

    methods: {
        async getBuckets() {
            const client = getClient();
            this.buckets = await client.getUserBuckets(this.userId);
            this.bucketIds = this.buckets.map(b => b.bid);
            this.loaded = true;
        }
    }
}
</script>