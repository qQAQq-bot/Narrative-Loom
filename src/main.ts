import { createApp } from "vue";
import { createRouter, createWebHashHistory } from "vue-router";
import { createPinia } from "pinia";
import App from "./App.vue";
import "./style.css";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/",
      name: "library",
      component: () => import("./views/LibraryView.vue"),
    },
    {
      path: "/book/:id",
      name: "book",
      component: () => import("./views/BookView.vue"),
      redirect: (to) => ({ name: "book-chapter", params: to.params }),
      children: [
        {
          path: "",
          name: "book-chapter",
          component: () => import("./views/ChapterView.vue"),
        },
        {
          path: "inbox",
          name: "book-inbox",
          component: () => import("./views/InboxView.vue"),
        },
        {
          path: "bible",
          name: "book-bible",
          component: () => import("./views/BibleView.vue"),
        },
        {
          path: "techniques",
          name: "book-techniques",
          component: () => import("./views/TechniqueView.vue"),
        },
        {
          path: "search",
          name: "book-search",
          component: () => import("./views/SearchView.vue"),
        },
      ],
    },
    {
      path: "/settings",
      name: "settings",
      component: () => import("./views/SettingsView.vue"),
    },
  ],
});

const pinia = createPinia();
const app = createApp(App);

app.use(router);
app.use(pinia);
app.mount("#app");
