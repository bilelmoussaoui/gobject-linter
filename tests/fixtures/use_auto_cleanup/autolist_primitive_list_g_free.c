// Should NOT trigger: list of char* primitives with g_free destructor
void test_primitive_list(void) {
    GSList *my_list = NULL;
    my_list = g_slist_prepend(my_list, g_strdup("hello"));
    my_list = g_slist_prepend(my_list, g_strdup("world"));

    g_slist_free_full(my_list, g_free);
}
