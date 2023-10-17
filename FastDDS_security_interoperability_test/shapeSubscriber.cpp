#include "shapeSubscriber.hpp"
#include "shapePubSubTypes.h"

#include <fastdds/dds/domain/DomainParticipantFactory.hpp>
#include <fastdds/dds/subscriber/Subscriber.hpp>
#include <fastdds/dds/subscriber/qos/SubscriberQos.hpp>
#include <fastdds/dds/subscriber/DataReader.hpp>
#include <fastdds/dds/subscriber/qos/DataReaderQos.hpp>

#include <unistd.h>
#include <signal.h>

using namespace eprosima::fastdds::dds;

ShapeSubscriber::ShapeSubscriber() : participant_(nullptr), subscriber_(nullptr), topic_(nullptr), reader_(nullptr), type_(new ShapePubSubType()), listener_() {}

ShapeSubscriber::~ShapeSubscriber()
{
    if (reader_ && subscriber_)
    {
        subscriber_->delete_datareader(reader_);
    }

    if (subscriber_ && participant_)
    {
        participant_->delete_subscriber(subscriber_);
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

bool ShapeSubscriber::init()
{
    DomainParticipantQos participant_qos;
    participant_qos.name("subscriber_participant");
    participant_ = DomainParticipantFactory::get_instance()->create_participant(0, participant_qos);

    if (participant_)
    {
        type_.register_type(participant_);
    }

    SubscriberQos subscriber_qos = SUBSCRIBER_QOS_DEFAULT;

    if (participant_)
    {
        subscriber_ = participant_->create_subscriber(subscriber_qos);
    }

    TopicQos topic_qos = TOPIC_QOS_DEFAULT;

    if (participant_)
    {
        topic_ = participant_->create_topic("Square", type_.get_type_name(), topic_qos);
    }

    DataReaderQos datareader_qos = DATAREADER_QOS_DEFAULT;

    if (subscriber_ && topic_)
    {
        reader_ = subscriber_->create_datareader(topic_, datareader_qos, &listener_);
    }

    if (reader_ && topic_ && subscriber_ && participant_)
    {
        std::cout << "DataReader created for the topic Square." << std::endl;
        return true;
    }
    else
    {
        return false;
    }
}

// For handling stop signal to break the infinite loop
namespace subscriber_stop{
volatile sig_atomic_t stop;
void handle_interrupt(int)
{
    stop = 1;
}}

void ShapeSubscriber::run()
{
    signal(SIGINT, subscriber_stop::handle_interrupt);

    std::cout << "Waiting for data" << std::endl;

    while (!subscriber_stop::stop)
    {
        std::this_thread::sleep_for(std::chrono::milliseconds(500));
    }
    std::cout << "\nStopped" << std::endl;
}

void ShapeSubscriber::SubscriberListener::on_data_available(DataReader *reader)
{
    Shape sample;
    SampleInfo info;

    ReturnCode_t return_code = reader->take_next_sample(&sample, &info);
    if (return_code == ReturnCode_t::RETCODE_OK)
    {
        ++received_samples;
         std::cout << "Sample received ["<<sample.color()<<": ("<<sample.x()<<","<<sample.y()<<")], count=" << received_samples << std::endl;
    }
    else
    {
        std::cout << "Read failed: return code " << return_code() << std::endl;
    }
}

void ShapeSubscriber::SubscriberListener::on_subscription_matched(DataReader *, const SubscriptionMatchedStatus &info)
{
    matched = info.current_count;
    std::cout << "Number of matched writers matched: " << matched << std::endl;
}