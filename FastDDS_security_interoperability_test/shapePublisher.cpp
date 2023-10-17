#include "shapePublisher.hpp"
#include "shapePubSubTypes.h"

#include <fastdds/dds/domain/DomainParticipantFactory.hpp>
#include <fastdds/dds/publisher/Publisher.hpp>
#include <fastdds/dds/publisher/qos/PublisherQos.hpp>
#include <fastdds/dds/publisher/DataWriter.hpp>
#include <fastdds/dds/publisher/qos/DataWriterQos.hpp>

#include <unistd.h>
#include <signal.h>

using namespace eprosima::fastdds::dds;

ShapePublisher::ShapePublisher() : participant_(nullptr), publisher_(nullptr), topic_(nullptr), writer_(nullptr), type_(new ShapePubSubType()), listener_() {}

ShapePublisher::~ShapePublisher()
{
    if (writer_ && publisher_)
    {
        publisher_->delete_datawriter(writer_);
    }

    if (publisher_ && participant_)
    {
        participant_->delete_publisher(publisher_);
    }

    if (topic_ && participant_)
    {
        participant_->delete_topic(topic_);
    }
    if (participant_)
    {
        DomainParticipantFactory::get_instance()->delete_participant(participant_);
    }
}

bool ShapePublisher::init()
{
    DomainParticipantQos participant_qos;
    participant_qos.name("publisher_participant");

    participant_ = DomainParticipantFactory::get_instance()->create_participant(0, participant_qos);

    if (participant_)
    {
        type_.register_type(participant_);
    }

    PublisherQos publisher_qos = PUBLISHER_QOS_DEFAULT;

    if (participant_)
    {
        publisher_ = participant_->create_publisher(publisher_qos);
    }

    TopicQos topic_qos = TOPIC_QOS_DEFAULT;

    if (participant_)
    {
        topic_ = participant_->create_topic("Square", type_.get_type_name(), topic_qos);
    }

    DataWriterQos datawriter_qos = DATAWRITER_QOS_DEFAULT;

    if (publisher_ && topic_)
    {
        writer_ = publisher_->create_datawriter(topic_, datawriter_qos, &listener_);
    }

    if (writer_ && topic_ && publisher_ && participant_)
    {
        std::cout << "DataWriter created for the topic Square." << std::endl;
        return true;
    }
    else
    {
        return false;
    }
}

// For handling stop signal to break the infinite loop
namespace publisher_stop
{
    volatile sig_atomic_t stop;
    void handle_interrupt(int)
    {
        stop = 1;
    }
}

void ShapePublisher::run()
{
    signal(SIGINT, publisher_stop::handle_interrupt);

    Shape sample;
    sample.color("BLUE");
    sample.shape_size(12);

    int number_of_messages_sent = 0;

    while (!publisher_stop::stop)
    {
        if (listener_.matched)
        {
            sample.x(10 * (number_of_messages_sent % 10));
            sample.y(10 * (number_of_messages_sent % 9));
            writer_->write(&sample);
            std::cout << "Sending sample " << number_of_messages_sent << std::endl;
            ++number_of_messages_sent;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(500));
    }
    std::cout << "\nStopped" << std::endl;
}

void ShapePublisher::SubscriberListener::on_publication_matched(DataWriter *, const PublicationMatchedStatus &info)
{
    matched = info.current_count;
    std::cout << "Number of matched readers: " << matched << std::endl;
}