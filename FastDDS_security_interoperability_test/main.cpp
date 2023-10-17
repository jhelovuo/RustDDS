#include "shapePublisher.hpp"
#include "shapeSubscriber.hpp"

#include <string.h>
#include <iostream>

int main(int number_of_arguments, char **argument_values)
{
    if (number_of_arguments != 2 || (strcmp(argument_values[1], "p") && strcmp(argument_values[1], "s")))
    {
        std::cout << "Usage: " << argument_values[0] << " p|s" << std::endl;
        return 0;
    }

    std::cout << "Start ";

    if (strcmp(argument_values[1], "p"))
    {
        std::cout << "subscriber" << std::endl;

        ShapeSubscriber subscriber;
        if (subscriber.init())
        {
            subscriber.run();
        };
    }
    else
    {
        std::cout << "publisher" << std::endl;

        ShapePublisher publisher;
        if (publisher.init())
        {
            publisher.run();
        };
    }
    return 0;
}