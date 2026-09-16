<script lang="ts">
  import { page } from "$app/state";
  import PersonPage from "$lib/PersonPage.svelte";
</script>

{#key page.params.id}<PersonPage personId={page.params.id!} />{/key}
