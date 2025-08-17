<script setup lang="ts">
import "@/render/image/index";

</script>

<template>
    <canvas id="canvas"></canvas>
</template>


<style>
@import url("./image.css");

body {
    margin: 0;
}

canvas {
    width: 100vw;
    height: 100vh;
    display: block;
}
</style>