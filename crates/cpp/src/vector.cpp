#include <vector>

using namespace std;

extern "C"
{
  // Create a new empty C++ vector
  void *cpp_create_vector()
  {
    vector<void *> *v = new vector<void *>();
    return v;
  }

  // Delete a C++ vector
  void cpp_delete_vector(void *ptr)
  {
    vector<void *> *v = (vector<void *> *)ptr;
    delete v;
  }

  // Get the size of the C++ vector
  size_t cpp_vector_size(void *ptr)
  {
    vector<void *> *v = (vector<void *> *)ptr;
    return v->size();
  }

  // Get the element at 'index' from the C++ vector
  void *cpp_vector_get(void *ptr, size_t index)
  {
    vector<void *> *v = (vector<void *> *)ptr;
    return (*v)[index];
  }

  // Clear the C++ vector
  void cpp_vector_clear(void *ptr)
  {
    vector<void *> *v = (vector<void *> *)ptr;
    v->clear();
  }

  // Push a new element to the C++ vector
  void cpp_vector_push_back(void *ptr, void *element)
  {
    vector<void *> *v = (vector<void *> *)ptr;
    v->push_back(element);
  }
}
