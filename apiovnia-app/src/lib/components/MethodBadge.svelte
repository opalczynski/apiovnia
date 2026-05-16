<script lang="ts">
  type HttpMethod =
    | "GET"
    | "POST"
    | "PUT"
    | "PATCH"
    | "DELETE"
    | "HEAD"
    | "OPTIONS";

  const { method }: { method: HttpMethod | string } = $props();
</script>

<span class="ap-method {method}">{method}</span>
