import "vue-router";

declare module "vue-router" {
  interface RouteMeta {
    /** When true, page renders without dashboard chrome (e.g. login). */
    public?: boolean;
  }
}
